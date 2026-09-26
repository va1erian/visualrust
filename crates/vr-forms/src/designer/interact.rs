//! Pure geometry for the designer: hit-testing, grid snapping, moving and
//! resizing.
//!
//! Nothing here names `xui` or a window, so the editing maths is unit tested
//! without a desktop. The caller converts device pixels to design units before
//! calling in, and every result is in design units.

use crate::model::{Bounds, Dip, Form, Size};

/// A point on the form, in design units relative to the form origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignPoint {
    pub x: f64,
    pub y: f64,
}

impl DesignPoint {
    pub const fn new(x: f64, y: f64) -> DesignPoint {
        DesignPoint { x, y }
    }
}

/// One of the eight resize handles drawn around the selected control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl Handle {
    /// Every handle, corners first so a corner wins a hit-test against an
    /// adjacent edge handle.
    pub const ALL: [Handle; 8] = [
        Handle::NorthWest,
        Handle::NorthEast,
        Handle::SouthEast,
        Handle::SouthWest,
        Handle::North,
        Handle::East,
        Handle::South,
        Handle::West,
    ];

    /// Whether the handle moves the rectangle's left edge.
    pub const fn moves_west(self) -> bool {
        matches!(self, Handle::NorthWest | Handle::West | Handle::SouthWest)
    }

    /// Whether the handle moves the rectangle's right edge.
    pub const fn moves_east(self) -> bool {
        matches!(self, Handle::NorthEast | Handle::East | Handle::SouthEast)
    }

    /// Whether the handle moves the rectangle's top edge.
    pub const fn moves_north(self) -> bool {
        matches!(self, Handle::NorthWest | Handle::North | Handle::NorthEast)
    }

    /// Whether the handle moves the rectangle's bottom edge.
    pub const fn moves_south(self) -> bool {
        matches!(self, Handle::SouthWest | Handle::South | Handle::SouthEast)
    }

    /// The handle's anchor point on `bounds`, in design units.
    pub fn anchor(self, bounds: Bounds) -> DesignPoint {
        let left = bounds.x.get();
        let top = bounds.y.get();
        let right = left + bounds.width.get();
        let bottom = top + bounds.height.get();
        let mid_x = (left + right) / 2.0;
        let mid_y = (top + bottom) / 2.0;
        let (x, y) = match self {
            Handle::NorthWest => (left, top),
            Handle::North => (mid_x, top),
            Handle::NorthEast => (right, top),
            Handle::East => (right, mid_y),
            Handle::SouthEast => (right, bottom),
            Handle::South => (mid_x, bottom),
            Handle::SouthWest => (left, bottom),
            Handle::West => (left, mid_y),
        };
        DesignPoint::new(x, y)
    }
}

/// The topmost control whose bounds contain `point`, or `None` for the form
/// itself. `controls` is z-order, so the last match wins.
pub fn hit_test(form: &Form, point: DesignPoint) -> Option<usize> {
    form.controls
        .iter()
        .enumerate()
        .rev()
        .find(|(_, control)| contains(control.bounds, point))
        .map(|(index, _)| index)
}

/// Whether `bounds` contains `point`, treating the right/bottom edges as
/// exclusive so adjacent controls never both claim a boundary pixel.
fn contains(bounds: Bounds, point: DesignPoint) -> bool {
    point.x >= bounds.x.get()
        && point.x < bounds.x.get() + bounds.width.get()
        && point.y >= bounds.y.get()
        && point.y < bounds.y.get() + bounds.height.get()
}

/// The resize handle within `reach` design units of `point`, if any.
pub fn handle_at(bounds: Bounds, point: DesignPoint, reach: f64) -> Option<Handle> {
    Handle::ALL.into_iter().find(|handle| {
        let anchor = handle.anchor(bounds);
        (anchor.x - point.x).abs() <= reach && (anchor.y - point.y).abs() <= reach
    })
}

/// Rounds `value` to the nearest multiple of `grid`. A non-positive grid
/// disables snapping.
pub fn snap(value: f64, grid: f64) -> f64 {
    if grid <= 0.0 {
        value
    } else {
        (value / grid).round() * grid
    }
}

/// Snaps every edge of `bounds` to the grid, keeping a minimum extent of one
/// grid step so a control cannot collapse to nothing.
pub fn snap_bounds(bounds: Bounds, grid: f64) -> Bounds {
    let floor = if grid > 0.0 { grid } else { 0.0 };
    Bounds {
        x: Dip::new(snap(bounds.x.get(), grid)),
        y: Dip::new(snap(bounds.y.get(), grid)),
        width: Dip::new(snap(bounds.width.get(), grid).max(floor)),
        height: Dip::new(snap(bounds.height.get(), grid).max(floor)),
    }
}

/// Moves `origin` by `delta` and snaps its position to the grid, clamped so the
/// control stays inside a `form`-sized surface.
pub fn moved_bounds(origin: Bounds, delta: DesignPoint, grid: f64, form: Size) -> Bounds {
    let max_x = (form.width.get() - origin.width.get()).max(0.0);
    let max_y = (form.height.get() - origin.height.get()).max(0.0);
    Bounds {
        x: Dip::new(snap(origin.x.get() + delta.x, grid).clamp(0.0, max_x)),
        y: Dip::new(snap(origin.y.get() + delta.y, grid).clamp(0.0, max_y)),
        ..origin
    }
}

/// Resizes `origin` by dragging `handle` to `pointer`, snapping the moved edges
/// to the grid. The non-moving edges stay put and the result keeps at least
/// `min_size` design units in each dimension.
pub fn resized_bounds(
    origin: Bounds,
    handle: Handle,
    pointer: DesignPoint,
    grid: f64,
    min_size: f64,
) -> Bounds {
    let mut left = origin.x.get();
    let mut top = origin.y.get();
    let mut right = left + origin.width.get();
    let mut bottom = top + origin.height.get();

    if handle.moves_west() {
        left = snap(pointer.x, grid);
    }
    if handle.moves_east() {
        right = snap(pointer.x, grid);
    }
    if handle.moves_north() {
        top = snap(pointer.y, grid);
    }
    if handle.moves_south() {
        bottom = snap(pointer.y, grid);
    }

    // Push the dragged edge back so the rectangle never inverts or vanishes.
    if right - left < min_size {
        if handle.moves_west() {
            left = right - min_size;
        } else {
            right = left + min_size;
        }
    }
    if bottom - top < min_size {
        if handle.moves_north() {
            top = bottom - min_size;
        } else {
            bottom = top + min_size;
        }
    }

    Bounds {
        x: Dip::new(left),
        y: Dip::new(top),
        width: Dip::new(right - left),
        height: Dip::new(bottom - top),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Control, ControlKind};

    fn control(name: &str, x: f64, y: f64, w: f64, h: f64) -> Control {
        Control {
            kind: ControlKind::Button,
            name: name.to_string(),
            bounds: Bounds {
                x: Dip::new(x),
                y: Dip::new(y),
                width: Dip::new(w),
                height: Dip::new(h),
            },
            text: String::new(),
            enabled: true,
            visible: true,
            tooltip: None,
            anchor: crate::model::Anchor::TopLeft,
        }
    }

    fn form() -> Form {
        Form {
            name: "Form1".into(),
            size: Size {
                width: Dip::new(400.0),
                height: Dip::new(300.0),
            },
            controls: vec![
                control("lower", 10.0, 10.0, 100.0, 50.0),
                control("upper", 50.0, 20.0, 100.0, 50.0),
            ],
        }
    }

    #[test]
    fn hit_test_prefers_the_topmost_overlapping_control() {
        let form = form();
        // Inside both; the later control is on top.
        assert_eq!(hit_test(&form, DesignPoint::new(60.0, 30.0)), Some(1));
        // Only the lower one.
        assert_eq!(hit_test(&form, DesignPoint::new(15.0, 15.0)), Some(0));
        // Empty surface.
        assert_eq!(hit_test(&form, DesignPoint::new(399.0, 299.0)), None);
    }

    #[test]
    fn hit_test_treats_the_far_edge_as_exclusive() {
        let form = form();
        // (110, 15) is the lower control's right edge, above the upper one.
        assert_eq!(hit_test(&form, DesignPoint::new(110.0, 15.0)), None);
    }

    #[test]
    fn handle_at_finds_the_nearest_corner() {
        let bounds = Bounds {
            x: Dip::new(10.0),
            y: Dip::new(10.0),
            width: Dip::new(100.0),
            height: Dip::new(50.0),
        };
        assert_eq!(
            handle_at(bounds, DesignPoint::new(11.0, 11.0), 6.0),
            Some(Handle::NorthWest)
        );
        assert_eq!(
            handle_at(bounds, DesignPoint::new(110.0, 60.0), 6.0),
            Some(Handle::SouthEast)
        );
        assert_eq!(
            handle_at(bounds, DesignPoint::new(60.0, 10.0), 6.0),
            Some(Handle::North)
        );
        assert_eq!(handle_at(bounds, DesignPoint::new(60.0, 30.0), 6.0), None);
    }

    #[test]
    fn snap_rounds_to_the_nearest_grid_step() {
        assert_eq!(snap(13.0, 8.0), 16.0);
        assert_eq!(snap(11.0, 8.0), 8.0);
        assert_eq!(snap(13.0, 0.0), 13.0);
    }

    #[test]
    fn snap_bounds_keeps_a_minimum_extent() {
        let moved = snap_bounds(control("c", 3.0, 5.0, 20.0, 12.0).bounds, 8.0);
        assert_eq!((moved.x.get(), moved.y.get()), (0.0, 8.0));
        assert_eq!((moved.width.get(), moved.height.get()), (24.0, 16.0));
    }

    #[test]
    fn moved_bounds_snaps_and_clamps_inside_the_form() {
        let origin = control("c", 10.0, 10.0, 100.0, 50.0).bounds;
        let form = form().size;
        let moved = moved_bounds(origin, DesignPoint::new(95.0, -30.0), 8.0, form);
        // x = 10 + 95 = 105 snapped to 104, clamped to 300; y = -20 snapped to
        // -24, clamped to 0.
        assert_eq!((moved.x.get(), moved.y.get()), (104.0, 0.0));
        assert_eq!((moved.width.get(), moved.height.get()), (100.0, 50.0));
    }

    #[test]
    fn resize_from_the_south_east_grows_the_rectangle() {
        let origin = control("c", 10.0, 10.0, 100.0, 50.0).bounds;
        let resized = resized_bounds(
            origin,
            Handle::SouthEast,
            DesignPoint::new(207.0, 133.0),
            8.0,
            8.0,
        );
        // right = snap(207) = 208, bottom = snap(133) = 136.
        assert_eq!(resized.x.get(), 10.0);
        assert_eq!(resized.y.get(), 10.0);
        assert_eq!(resized.width.get(), 198.0);
        assert_eq!(resized.height.get(), 126.0);
    }

    #[test]
    fn resize_from_the_north_west_moves_the_origin() {
        let origin = control("c", 100.0, 100.0, 100.0, 50.0).bounds;
        let resized = resized_bounds(
            origin,
            Handle::NorthWest,
            DesignPoint::new(58.0, 82.0),
            8.0,
            8.0,
        );
        assert_eq!((resized.x.get(), resized.y.get()), (56.0, 80.0));
        assert_eq!((resized.width.get(), resized.height.get()), (144.0, 70.0));
    }

    #[test]
    fn resize_never_collapses_below_the_minimum() {
        let origin = control("c", 10.0, 10.0, 100.0, 50.0).bounds;
        let resized = resized_bounds(
            origin,
            Handle::SouthEast,
            DesignPoint::new(2.0, 4.0),
            8.0,
            8.0,
        );
        assert_eq!(resized.width.get(), 8.0);
        assert_eq!(resized.height.get(), 8.0);
    }
}
