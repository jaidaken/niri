//! Scroll-axis abstraction for orientation-aware layout.
//!
//! niri scrolls columns horizontally by default: columns advance along X and the
//! view scrolls along X, while tiles stack down Y inside a column. To support
//! per-output vertical scrolling, layout geometry is expressed in main/cross terms
//! and mapped to screen x/y through [`ScrollAxis`]. `main` is the axis columns
//! advance along (and the view scrolls along); `cross` is the axis tiles stack
//! along within a column. Workspaces switch along the perpendicular of `main`.

use smithay::utils::{Coordinate, Point, Size};

/// Orientation of a scrollable space: which screen axis columns scroll along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollAxis {
    /// Columns scroll left/right along X; tiles stack down Y. niri's default.
    #[default]
    Horizontal,
    /// Columns scroll up/down along Y; tiles stack across X.
    Vertical,
}

impl ScrollAxis {
    /// Main-axis (scroll direction) component of a point.
    pub fn main<N: Coordinate, K>(self, p: Point<N, K>) -> N {
        match self {
            ScrollAxis::Horizontal => p.x,
            ScrollAxis::Vertical => p.y,
        }
    }

    /// Cross-axis (tile stacking) component of a point.
    pub fn cross<N: Coordinate, K>(self, p: Point<N, K>) -> N {
        match self {
            ScrollAxis::Horizontal => p.y,
            ScrollAxis::Vertical => p.x,
        }
    }

    /// Build a point from main (scroll) and cross (stack) components.
    pub fn point<N: Coordinate, K>(self, main: N, cross: N) -> Point<N, K> {
        match self {
            ScrollAxis::Horizontal => Point::from((main, cross)),
            ScrollAxis::Vertical => Point::from((cross, main)),
        }
    }

    /// Main-axis extent of a size: column width when horizontal, height when vertical.
    pub fn main_size<N: Coordinate, K>(self, s: Size<N, K>) -> N {
        match self {
            ScrollAxis::Horizontal => s.w,
            ScrollAxis::Vertical => s.h,
        }
    }

    /// Cross-axis extent of a size.
    pub fn cross_size<N: Coordinate, K>(self, s: Size<N, K>) -> N {
        match self {
            ScrollAxis::Horizontal => s.h,
            ScrollAxis::Vertical => s.w,
        }
    }

    /// Build a size from main and cross extents.
    pub fn size<N: Coordinate, K>(self, main: N, cross: N) -> Size<N, K> {
        match self {
            ScrollAxis::Horizontal => Size::from((main, cross)),
            ScrollAxis::Vertical => Size::from((cross, main)),
        }
    }

    /// The perpendicular axis. Workspaces switch along this when columns scroll along `self`.
    pub fn perpendicular(self) -> Self {
        match self {
            ScrollAxis::Horizontal => ScrollAxis::Vertical,
            ScrollAxis::Vertical => ScrollAxis::Horizontal,
        }
    }
}
