//! Conversions between the model's design units and the device pixels the xui
//! backend paints in.
//!
//! Kept in one place so every caller scales once: a `Dip` that leaks into a
//! pixel coordinate, or vice versa, is the classic DPI bug.

use xui::{Rect, dip};

use crate::model::{Bounds, Size};

/// A model `Bounds` as the backend's pixel rectangle at `dpi`.
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

/// A form `Size` as the backend's pixel rectangle at `dpi`, at the origin.
pub fn size_to_rect(size: Size, dpi: u32) -> Rect {
    Rect::new(
        0,
        0,
        dip(size.width.get() as f32).to_px(dpi).value(),
        dip(size.height.get() as f32).to_px(dpi).value(),
    )
}

/// A device-pixel coordinate as design units at `dpi`.
pub fn px_to_dip(value: i32, dpi: u32) -> f64 {
    if dpi == 0 {
        value as f64
    } else {
        value as f64 * 96.0 / dpi as f64
    }
}

/// A design value as device pixels at `dpi`.
pub fn dip_to_px(value: f64, dpi: u32) -> i32 {
    (value * dpi as f64 / 96.0).round() as i32
}
