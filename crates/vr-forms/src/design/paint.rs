//! Painting the design surface: the form page, the grid, a themed proxy box
//! per control and the selection outline with its eight handles.
//!
//! Everything is drawn by the one [`Custom`](xui::Custom) widget: unlike a
//! surface that hosts live child widgets, the designer works on a machine with
//! no interactive desktop and inside any hosting app, because there is no child
//! control to create. Coordinates are the widget's client pixels; the model is
//! in design units, so the page is scaled once through
//! [`units`](super::units).

use xui::gdi::{Canvas, Font, TextFormat};
use xui::{Rect, Theme};

use crate::model::Control;

use super::interact::Handle;
use super::units::{bounds_to_rect, dip_to_px, size_to_rect};
use super::widget::DesignerState;

/// The on-screen size of one resize handle, in design units.
const HANDLE_DIP: f64 = 7.0;

/// The width of the selection outline, in pixels.
const OUTLINE_PX: i32 = 2;

/// Paints the whole design surface.
pub(crate) fn paint(
    canvas: &Canvas,
    bounds: Rect,
    theme: &Theme,
    state: &DesignerState,
    font: Option<&Font>,
    dpi: u32,
) {
    canvas.fill_rect(bounds, theme.background);
    let page = size_to_rect(state.form.size, dpi);
    canvas.fill_rect(page, theme.raised);
    paint_grid(canvas, theme, page, state.grid, dpi);

    for control in &state.form.controls {
        let rect = bounds_to_rect(control.bounds, dpi);
        paint_proxy(canvas, theme, rect, control, font, dpi);
    }

    if let Some(index) = state.selected
        && let Some(control) = state.form.controls.get(index)
    {
        paint_selection(canvas, theme, bounds_to_rect(control.bounds, dpi), dpi);
    }
}

/// Draws a grid line every `grid` design units inside the page.
fn paint_grid(canvas: &Canvas, theme: &Theme, page: Rect, grid: f64, dpi: u32) {
    // `grid <= 0` disables snapping; drawing at a minimum 1px step would then
    // fill the surface and cost a draw call per row and column.
    if grid <= 0.0 {
        return;
    }
    let step = dip_to_px(grid, dpi).max(1);
    let mut x = page.left + step;
    while x < page.right {
        canvas.line(
            xui::Point::new(x, page.top),
            xui::Point::new(x, page.bottom),
            theme.border,
            1,
        );
        x += step;
    }
    let mut y = page.top + step;
    while y < page.bottom {
        canvas.line(
            xui::Point::new(page.left, y),
            xui::Point::new(page.right, y),
            theme.border,
            1,
        );
        y += step;
    }
}

/// Draws one control as a themed proxy box captioned with its name and kind.
fn paint_proxy(
    canvas: &Canvas,
    theme: &Theme,
    rect: Rect,
    control: &Control,
    font: Option<&Font>,
    dpi: u32,
) {
    if rect.is_empty() {
        return;
    }
    let fill = if control.visible {
        theme.surface
    } else {
        theme.background
    };
    canvas.fill_rect(rect, fill);
    outline(canvas, rect, theme.border, 1);

    let caption = format!("{} [{}]", control.name, control.kind.tag());
    let style = TextFormat::left()
        .vcenter()
        .single_line()
        .end_ellipsis()
        .no_prefix();
    let text_rect = rect.shrink(dip_to_px(4.0, dpi).max(2));
    let color = if control.enabled {
        theme.text
    } else {
        theme.text_disabled
    };
    match font {
        Some(font) => {
            canvas.with_font(font, |canvas| {
                canvas.draw_text(text_rect, &caption, color, style);
            });
        }
        None => {
            canvas.draw_text(text_rect, &caption, color, style);
        }
    }
}

/// Draws the accent outline and eight handles around the selected control.
fn paint_selection(canvas: &Canvas, theme: &Theme, base: Rect, dpi: u32) {
    let handle = dip_to_px(HANDLE_DIP, dpi).max(HANDLE_DIP as i32);
    let margin = handle / 2 + OUTLINE_PX;
    let adorn = Rect::new(
        base.left - margin,
        base.top - margin,
        base.right + margin,
        base.bottom + margin,
    );
    outline(canvas, adorn, theme.accent, OUTLINE_PX);
    for corner in Handle::ALL {
        let (x, y) = anchor(adorn, corner);
        let square = Rect::new(
            x - handle / 2,
            y - handle / 2,
            x + handle / 2,
            y + handle / 2,
        );
        canvas.fill_rect(square, theme.accent);
        outline(canvas, square, theme.background, 1);
    }
}

/// Draws a `width`-pixel outline just inside `rect` by stacking 1-pixel
/// outlines, since the canvas offers only a single-pixel one.
fn outline(canvas: &Canvas, rect: Rect, color: xui::Color, width: i32) {
    for inset in 0..width {
        canvas.outline(rect.shrink(inset), color);
    }
}

/// The pixel position of `handle` on `rect`, mirroring
/// [`Handle::anchor`](super::interact::Handle::anchor).
fn anchor(rect: Rect, handle: Handle) -> (i32, i32) {
    let mid_x = (rect.left + rect.right) / 2;
    let mid_y = (rect.top + rect.bottom) / 2;
    match handle {
        Handle::NorthWest => (rect.left, rect.top),
        Handle::North => (mid_x, rect.top),
        Handle::NorthEast => (rect.right, rect.top),
        Handle::East => (rect.right, mid_y),
        Handle::SouthEast => (rect.right, rect.bottom),
        Handle::South => (mid_x, rect.bottom),
        Handle::SouthWest => (rect.left, rect.bottom),
        Handle::West => (rect.left, mid_y),
    }
}
