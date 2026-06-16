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

    /// Return `p` with `delta` added to its main-axis (scroll) component.
    pub fn add_main<N: Coordinate, K>(self, p: Point<N, K>, delta: N) -> Point<N, K> {
        self.point(self.main(p) + delta, self.cross(p))
    }

    /// Return `p` with `delta` added to its cross-axis (stacking) component.
    pub fn add_cross<N: Coordinate, K>(self, p: Point<N, K>, delta: N) -> Point<N, K> {
        self.point(self.main(p), self.cross(p) + delta)
    }
}

#[cfg(test)]
mod tests {
    use smithay::utils::{Logical, Point, Size};

    use super::ScrollAxis;

    fn pt(x: f64, y: f64) -> Point<f64, Logical> {
        Point::from((x, y))
    }

    fn sz(w: f64, h: f64) -> Size<f64, Logical> {
        Size::from((w, h))
    }

    #[test]
    fn horizontal_main_is_x_cross_is_y() {
        let p = pt(3.0, 7.0);
        assert_eq!(ScrollAxis::Horizontal.main(p), 3.0);
        assert_eq!(ScrollAxis::Horizontal.cross(p), 7.0);
    }

    #[test]
    fn vertical_main_is_y_cross_is_x() {
        let p = pt(3.0, 7.0);
        assert_eq!(ScrollAxis::Vertical.main(p), 7.0);
        assert_eq!(ScrollAxis::Vertical.cross(p), 3.0);
    }

    #[test]
    fn horizontal_point_places_main_on_x() {
        assert_eq!(ScrollAxis::Horizontal.point(5.0, 9.0), pt(5.0, 9.0));
    }

    #[test]
    fn vertical_point_swaps_main_onto_y() {
        assert_eq!(ScrollAxis::Vertical.point(5.0, 9.0), pt(9.0, 5.0));
    }

    #[test]
    fn size_main_cross_track_orientation() {
        let s = sz(4.0, 6.0);
        assert_eq!(ScrollAxis::Horizontal.main_size(s), 4.0);
        assert_eq!(ScrollAxis::Horizontal.cross_size(s), 6.0);
        assert_eq!(ScrollAxis::Vertical.main_size(s), 6.0);
        assert_eq!(ScrollAxis::Vertical.cross_size(s), 4.0);
    }

    #[test]
    fn point_round_trips_through_main_cross_both_axes() {
        for axis in [ScrollAxis::Horizontal, ScrollAxis::Vertical] {
            let p: Point<f64, Logical> = axis.point(2.0, 8.0);
            assert_eq!((axis.main(p), axis.cross(p)), (2.0, 8.0));
            let s: Size<f64, Logical> = axis.size(2.0, 8.0);
            assert_eq!((axis.main_size(s), axis.cross_size(s)), (2.0, 8.0));
        }
    }

    #[test]
    fn add_main_moves_only_the_scroll_axis() {
        assert_eq!(ScrollAxis::Horizontal.add_main(pt(1.0, 1.0), 10.0), pt(11.0, 1.0));
        assert_eq!(ScrollAxis::Vertical.add_main(pt(1.0, 1.0), 10.0), pt(1.0, 11.0));
    }

    #[test]
    fn add_cross_moves_only_the_stacking_axis() {
        assert_eq!(ScrollAxis::Horizontal.add_cross(pt(1.0, 1.0), 10.0), pt(1.0, 11.0));
        assert_eq!(ScrollAxis::Vertical.add_cross(pt(1.0, 1.0), 10.0), pt(11.0, 1.0));
    }

    #[test]
    fn perpendicular_flips_and_is_involutive() {
        assert_eq!(ScrollAxis::Horizontal.perpendicular(), ScrollAxis::Vertical);
        assert_eq!(ScrollAxis::Vertical.perpendicular(), ScrollAxis::Horizontal);
        assert_eq!(
            ScrollAxis::Horizontal.perpendicular().perpendicular(),
            ScrollAxis::Horizontal
        );
    }

    #[test]
    fn default_axis_is_horizontal() {
        assert_eq!(ScrollAxis::default(), ScrollAxis::Horizontal);
    }
}
