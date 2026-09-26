//! Painting the design surface: the grid, placeholder boxes and the selection
//! adorners.
//!
//! Only the parts the surface owns are drawn here. A control with a portable
//! widget paints itself in its own child window; the surface draws underneath
//! it, so adorners are placed just outside the widget's bounds to stay visible.

use xui::xui_core::{Canvas, TextStyle};
use xui::{Point, Rect, Theme, dip};

use crate::designer::convert::{bounds_to_rect, dip_to_px};
use crate::designer::hosted::is_portable;
use crate::designer::interact::Handle;
use crate::model::Form;

/// The design size of placeholder and caption text.
const CAPTION_SIZE: f32 = 11.0;

/// Paints the whole surface: background, grid, placeholders and, when a control
/// is selected, its outline and eight handles.
///
/// `placeholders` is parallel to `form.controls`: `true` marks a control whose
/// widget could not be hosted (or has no portable widget), so it is drawn as a
/// labelled box instead.
pub fn paint_surface(
    canvas: &mut dyn Canvas,
    theme: &Theme,
    form: &Form,
    placeholders: &[bool],
    selected: Option<usize>,
    grid: f64,
    dpi: u32,
) {
    let bounds = canvas.bounds();
    canvas.clear(theme.background);
    paint_grid(canvas, theme, bounds, grid, dpi);

    for (index, control) in form.controls.iter().enumerate() {
        let unhosted = placeholders.get(index).copied().unwrap_or(false);
        if is_portable(&control.kind) && !unhosted {
            continue;
        }
        paint_placeholder(canvas, theme, control, dpi);
    }

    if let Some(index) = selected
        && let Some(control) = form.controls.get(index)
    {
        paint_selection(canvas, theme, bounds_to_rect(control.bounds, dpi), dpi);
    }
}

/// Draws a grid line every `grid` design units.
fn paint_grid(canvas: &mut dyn Canvas, theme: &Theme, bounds: Rect, grid: f64, dpi: u32) {
    // `grid <= 0` disables snapping; drawing at a minimum 1px step would then
    // fill the surface and cost a draw call per row and column.
    if grid <= 0.0 {
        return;
    }
    let step = dip_to_px(grid, dpi).max(1);
    let mut x = bounds.left;
    while x < bounds.right {
        canvas.draw_line(
            Point::new(x, bounds.top),
            Point::new(x, bounds.bottom),
            theme.border,
            1.0,
        );
        x += step;
    }
    let mut y = bounds.top;
    while y < bounds.bottom {
        canvas.draw_line(
            Point::new(bounds.left, y),
            Point::new(bounds.right, y),
            theme.border,
            1.0,
        );
        y += step;
    }
}

/// Draws a labelled box for a control with no portable widget.
fn paint_placeholder(
    canvas: &mut dyn Canvas,
    theme: &Theme,
    control: &crate::model::Control,
    dpi: u32,
) {
    let rect = bounds_to_rect(control.bounds, dpi);
    canvas.fill_rect(rect, theme.surface);
    canvas.stroke_rect(rect, theme.border, 1.0);
    let caption = if control.name.is_empty() {
        control.kind.tag().to_string()
    } else {
        format!("{} ({})", control.name, control.kind.tag())
    };
    let style = TextStyle::new(theme.text_secondary, dip(CAPTION_SIZE)).middle();
    canvas.draw_text(&caption, rect.shrink(dip_to_px(4.0, dpi)), &style);
}

/// Draws the outline and eight resize handles around the selected control.
fn paint_selection(canvas: &mut dyn Canvas, theme: &Theme, base: Rect, dpi: u32) {
    let handle = dip_to_px(7.0, dpi).max(7);
    let margin = handle / 2 + 1;
    let adorn = Rect::new(
        base.left - margin,
        base.top - margin,
        base.right + margin,
        base.bottom + margin,
    );
    canvas.stroke_rect(adorn, theme.accent, 2.0);
    for corner in Handle::ALL {
        let (x, y) = anchor(adorn, corner);
        let square = Rect::new(
            x - handle / 2,
            y - handle / 2,
            x + handle / 2,
            y + handle / 2,
        );
        canvas.fill_rect(square, theme.accent);
        canvas.stroke_rect(square, theme.background, 1.0);
    }
}

/// The pixel position of `handle` on `rect`, mirroring
/// [`Handle::anchor`](crate::designer::interact::Handle::anchor).
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use xui::xui_core::TextStyle;
    use xui::xui_core::backend::Canvas;
    use xui::{Color, Point, Rect};

    use crate::model::{Bounds, Control, ControlKind, Dip, Form, Size, TreeViewProps};

    /// A canvas that records which drawing verbs ran, so the surface logic can
    /// be tested without a window.
    #[derive(Default)]
    struct Recorder {
        ops: RefCell<Vec<&'static str>>,
        texts: RefCell<Vec<String>>,
    }

    impl Canvas for Recorder {
        fn dpi(&self) -> u32 {
            96
        }
        fn bounds(&self) -> Rect {
            Rect::new(0, 0, 560, 360)
        }
        fn clear(&mut self, _color: Color) {
            self.ops.borrow_mut().push("clear");
        }
        fn fill_rect(&mut self, _rect: Rect, _color: Color) {
            self.ops.borrow_mut().push("fill_rect");
        }
        fn fill_rounded_rect(&mut self, _rect: Rect, _radius: f32, _color: Color) {}
        fn fill_ellipse(&mut self, _center: Point, _rx: f32, _ry: f32, _color: Color) {}
        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {
            self.ops.borrow_mut().push("stroke_rect");
        }
        fn stroke_rounded_rect(&mut self, _r: Rect, _rad: f32, _c: Color, _w: f32) {}
        fn stroke_ellipse(&mut self, _c: Point, _rx: f32, _ry: f32, _c2: Color, _w: f32) {}
        fn draw_line(&mut self, _from: Point, _to: Point, _color: Color, _width: f32) {
            self.ops.borrow_mut().push("draw_line");
        }
        fn draw_text(&mut self, text: &str, _rect: Rect, _style: &TextStyle) {
            self.ops.borrow_mut().push("draw_text");
            self.texts.borrow_mut().push(text.to_string());
        }
        fn push_clip(&mut self, _rect: Rect) {}
        fn pop_clip(&mut self) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn set_translation(&mut self, _x: f32, _y: f32) {}
        fn set_scale_translate(&mut self, _scale: f32, _x: f32, _y: f32) {}
    }

    fn form(control: Control) -> Form {
        Form {
            name: "Paint".into(),
            size: Size {
                width: Dip::new(560.0),
                height: Dip::new(360.0),
            },
            controls: vec![control],
        }
    }

    fn tree() -> Control {
        Control {
            kind: ControlKind::TreeView(TreeViewProps::default()),
            name: "FilesTree".into(),
            bounds: Bounds {
                x: Dip::new(20.0),
                y: Dip::new(20.0),
                width: Dip::new(200.0),
                height: Dip::new(80.0),
            },
            text: String::new(),
            enabled: true,
            visible: true,
            tooltip: None,
            anchor: crate::model::Anchor::TopLeft,
        }
    }

    #[test]
    fn a_non_portable_control_paints_a_labelled_placeholder() {
        let mut canvas = Recorder::default();
        paint_surface(
            &mut canvas,
            &Theme::light(),
            &form(tree()),
            &[false],
            None,
            8.0,
            96,
        );
        assert!(canvas.ops.borrow().contains(&"fill_rect"));
        assert!(
            canvas
                .texts
                .borrow()
                .iter()
                .any(|text| text.contains("FilesTree") && text.contains("tree_view")),
            "the placeholder caption named the control and kind: {:?}",
            canvas.texts.borrow()
        );
    }

    #[test]
    fn a_portable_control_draws_no_placeholder() {
        let mut control = tree();
        control.kind = ControlKind::Button;
        let mut canvas = Recorder::default();
        paint_surface(
            &mut canvas,
            &Theme::light(),
            &form(control),
            &[false],
            None,
            8.0,
            96,
        );
        // The grid still draws lines, but no box or caption for the button.
        assert!(
            !canvas
                .texts
                .borrow()
                .iter()
                .any(|text| text.contains("button"))
        );
    }

    #[test]
    fn a_portable_control_that_failed_to_host_paints_a_placeholder() {
        // The native `Edit` on a machine without a desktop is the real case.
        let mut control = tree();
        control.kind = ControlKind::Edit(crate::model::EditProps::default());
        let mut canvas = Recorder::default();
        paint_surface(
            &mut canvas,
            &Theme::light(),
            &form(control),
            &[true],
            None,
            8.0,
            96,
        );
        assert!(
            canvas
                .texts
                .borrow()
                .iter()
                .any(|text| text.contains("FilesTree") && text.contains("edit")),
            "the fallback placeholder names the edit: {:?}",
            canvas.texts.borrow()
        );
    }

    #[test]
    fn a_selected_control_paints_eight_handles() {
        let mut canvas = Recorder::default();
        paint_surface(
            &mut canvas,
            &Theme::light(),
            &form(tree()),
            &[false],
            Some(0),
            8.0,
            96,
        );
        let fills = canvas
            .ops
            .borrow()
            .iter()
            .filter(|op| **op == "fill_rect")
            .count();
        // One placeholder fill plus eight handle fills.
        assert_eq!(fills, 9, "placeholder plus eight handles");
    }
}
