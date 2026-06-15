//! HDR10 connector signaling helpers (Colorspace + HDR_OUTPUT_METADATA).
//!
//! Ported from jibsta210/cosmic-comp feat/hdr-experiment drm_helpers.rs. The
//! blob layout pitfalls (32-byte struct, interleaved primaries, no Vec
//! indirection) are inherited from there; see the struct comment.

use anyhow::{anyhow, Context as _, Result};
use smithay::backend::drm::HdrState;
use smithay::reexports::drm::control::{connector, property, Device as ControlDevice};

/// Kernel UAPI `struct hdr_output_metadata` (drm_mode.h), byte-for-byte.
///
/// `#[repr(C)]`, NOT `packed`: the leading u32 forces 4-byte alignment, so the
/// 30 bytes of fields round up to the kernel's expected 32 via the explicit
/// `_padding`. The kernel rejects blobs whose length differs from 32.
/// `display_primaries` is interleaved `[r_x, r_y, g_x, g_y, b_x, b_y]` per the
/// kernel's `struct { u16 x, y; } display_primaries[3]`.
#[repr(C)]
#[derive(Clone, Copy)]
struct HdrOutputMetadata {
    metadata_type: u32,
    eotf: u8,
    static_metadata_type: u8,
    display_primaries: [u16; 6],
    white_point: [u16; 2],
    max_display_mastering_luminance: u16,
    min_display_mastering_luminance: u16,
    max_cll: u16,
    max_fall: u16,
    _padding: u16,
}

/// SMPTE ST 2084 (PQ).
const EOTF_PQ: u8 = 2;

// BT.2020 primaries in 0.00002 chromaticity units, interleaved.
const BT2020_PRIMARIES: [u16; 6] = [
    35400, 14600, // R: 0.708, 0.292
    8500, 39850, // G: 0.170, 0.797
    6550, 2300, // B: 0.131, 0.046
];
// D65 white point, same units.
const D65_WHITE: [u16; 2] = [15635, 16450];

/// Mastering/content luminance for the metadata blob, kernel units.
#[derive(Debug, Clone, Copy)]
pub struct HdrLuminance {
    /// Mastering display peak, cd/m^2.
    pub max_lum_nits: u16,
    /// Mastering display min, 0.0001 cd/m^2 units.
    pub min_lum_units: u16,
    /// Content peak pixel, cd/m^2.
    pub max_cll_nits: u16,
    /// Content frame average, cd/m^2.
    pub max_fall_nits: u16,
}

impl HdrLuminance {
    /// Content-aware values for an SDR desktop mapped to `ref_white` nits.
    /// MaxCLL = ref_white (content peak, NOT panel peak): overstating it makes
    /// panel firmware tone-map for bright HDR and visibly dims the desktop.
    pub fn for_sdr_content(ref_white_nits: u16) -> Self {
        Self {
            max_lum_nits: 400, // conservative HDR400-class mastering bound
            min_lum_units: 50, // 0.005 nits; LCD-safe, panels clamp upward
            max_cll_nits: ref_white_nits,
            max_fall_nits: ref_white_nits.saturating_mul(2) / 5,
        }
    }
}

fn find_prop(
    dev: &impl ControlDevice,
    conn: connector::Handle,
    name: &str,
) -> Result<(property::Handle, property::Info)> {
    let props = dev.get_properties(conn).context("getting properties")?;
    let (handles, _) = props.as_props_and_values();
    for handle in handles {
        let info = dev.get_property(*handle).context("getting property")?;
        if info.name().to_str().ok() == Some(name) {
            return Ok((*handle, info));
        }
    }
    Err(anyhow!("connector has no property {name:?}"))
}

/// Raw u64 of a `Colorspace` enum variant on this connector.
pub fn colorspace_enum_value(
    dev: &impl ControlDevice,
    conn: connector::Handle,
    variant_name: &str,
) -> Result<u64> {
    let (_, info) = find_prop(dev, conn, "Colorspace")?;
    let property::ValueType::Enum(variants) = info.value_type() else {
        return Err(anyhow!("Colorspace has wrong value type"));
    };
    variants
        .values()
        .1
        .iter()
        .find(|v| v.name().to_str().ok() == Some(variant_name))
        .map(|v| v.value())
        .ok_or_else(|| anyhow!("Colorspace enum has no variant {variant_name:?}"))
}

/// Does the connector expose the property surface HDR signaling needs?
pub fn connector_supports_hdr(dev: &impl ControlDevice, conn: connector::Handle) -> bool {
    find_prop(dev, conn, "HDR_OUTPUT_METADATA").is_ok()
        && colorspace_enum_value(dev, conn, "BT2020_RGB").is_ok()
}

/// Build the PQ/BT.2020 metadata blob; returns its raw blob ID. The struct is
/// passed directly (NOT via Vec) so `create_property_blob` blobs the 32 bytes
/// of metadata rather than a Vec header.
pub fn create_hdr_metadata_blob(dev: &impl ControlDevice, lum: HdrLuminance) -> Result<u64> {
    let metadata = HdrOutputMetadata {
        metadata_type: 0, // HDR_OUTPUT_METADATA_TYPE1
        eotf: EOTF_PQ,
        static_metadata_type: 0,
        display_primaries: BT2020_PRIMARIES,
        white_point: D65_WHITE,
        max_display_mastering_luminance: lum.max_lum_nits,
        min_display_mastering_luminance: lum.min_lum_units,
        max_cll: lum.max_cll_nits,
        max_fall: lum.max_fall_nits,
        _padding: 0,
    };
    let blob = dev
        .create_property_blob(&metadata)
        .context("create HDR_OUTPUT_METADATA blob")?;
    Ok(blob.into())
}

/// Resolve signaling-only HDR10 for this connector. Returns the `HdrState` to
/// stage plus the raw HDR_OUTPUT_METADATA blob ID, which the caller owns and
/// must `destroy_property_blob` on re-stage or teardown (blobs persist until
/// the DRM fd closes, drm_property.c).
pub fn signaling_state(
    dev: &impl ControlDevice,
    conn: connector::Handle,
    ref_white_nits: f64,
) -> Result<(HdrState, u64)> {
    if !connector_supports_hdr(dev, conn) {
        return Err(anyhow!("connector does not advertise HDR support"));
    }
    let colorspace = colorspace_enum_value(dev, conn, "BT2020_RGB")?;
    let ref_white = ref_white_nits.clamp(80., 1000.) as u16;
    let blob = create_hdr_metadata_blob(dev, HdrLuminance::for_sdr_content(ref_white))?;
    Ok((HdrState::signaling_only(colorspace, blob), blob))
}
