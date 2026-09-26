//! Conversions between the model's design units and the device pixels the
//! [`Custom`](xui::Custom) widget paints and receives input in.
//!
//! Kept in one place so a `Dip` that leaks into a pixel coordinate, or vice
//! versa, does not become a DPI bug scattered across the painter and the input
//! handlers.

use xui::Rect;
use xui::dip;

use crate::model::{Bounds, Size};

/// A model `Bounds` as a pixel rectangle at `dpi`.
pub fn bounds_to_rect(bounds: Bounds, dpi: u32) -> Rect {
    Rect::new(
        dip(bounds.x.get() as f32).to_px(dpi).value(),
        dip(bounds.y.get() as f32).to_px(dpi).value(),
        dip((bounds.x.get() + bounds.width.get()) as f32)
            .to_px(dpi)
            .value(),
        dip((bounds.y.get() + bounds.height.get()) as f32)
            .to_px(dpi)
            .value(),
    )
}

/// A form `Size` as a pixel rectangle at the origin at `dpi`.
pub fn size_to_rect(size: Size, dpi: u32) -> Rect {
    Rect::new(
        0,
        0,
        dip(size.width.get() as f32).to_px(dpi).value(),
        dip(size.height.get() as f32).to_px(dpi).value(),
    )
}

/// A device-pixel coordinate as design units at `dpi`.
///
/// A zero `dpi` (a session that never reported one) falls back to the identity
/// so the designer still edits, rather than dividing by zero.
pub fn px_to_dip(value: i32, dpi: u32) -> f64 {
    if dpi == 0 {
        value as f64
    } else {
        value as f64 * 96.0 / dpi as f64
    }
}

/// A design value as device pixels at `dpi`.
pub fn dip_to_px(value: f64, dpi: u32) -> i32 {
    if dpi == 0 {
        value.round() as i32
    } else {
        (value * dpi as f64 / 96.0).round() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bounds, Dip};

    #[test]
    fn at_96_dpi_pixels_and_design_units_coincide() {
        assert_eq!(px_to_dip(12, 96), 12.0);
        assert_eq!(dip_to_px(12.0, 96), 12);
    }

    #[test]
    fn bounds_round_trip_at_higher_dpi() {
        let bounds = Bounds {
            x: Dip::new(10.0),
            y: Dip::new(20.0),
            width: Dip::new(30.0),
            height: Dip::new(40.0),
        };
        let rect = bounds_to_rect(bounds, 192);
        assert_eq!(rect.left, 20);
        assert_eq!(rect.top, 40);
        assert_eq!(rect.right, 80);
        assert_eq!(rect.bottom, 120);
        assert_eq!(px_to_dip(rect.left, 192), 10.0);
    }
}
