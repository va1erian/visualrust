//! The [`Custom`](xui::Custom) widget that hosts the form designer.
//!
//! All of the designer's state lives in a [`DesignerState`] behind an
//! `Rc<RefCell<..>>`: [`CustomWidget`] methods take `&self`, so every edit,
//! selection change and drag is interior-mutable. The widget decodes typed
//! [`Input`] into design-unit coordinates, applies it through the pure
//! [`interact`](super::interact) maths, and raises a [`DesignerEvent`] so the
//! hosting app can persist the form or refresh its own views.

use std::cell::RefCell;
use std::rc::Rc;

use xui::gdi::{Canvas, Font};
use xui::{CustomWidget, Input, Key, KeyResult, MouseButton, Rect, Size, Theme, WidgetCx};

use crate::model::{Bounds, Form, apply_anchors};

use super::DesignerEvent;
use super::interact::{self, DesignPoint, Handle};
use super::paint;
use super::units::{px_to_dip, size_to_rect};

/// The smallest a resize may leave a control, in design units.
const MIN_SIZE_DIP: f64 = 8.0;

/// How close, in design units, a pointer must be to a handle to grab it.
const HANDLE_REACH_DIP: f64 = 6.0;

/// A drag in progress, in design units.
enum Drag {
    Move { start: DesignPoint, origin: Bounds },
    Resize { handle: Handle, origin: Bounds },
}

/// The form model plus the transient selection and drag state.
pub(crate) struct DesignerState {
    pub(crate) form: Form,
    pub(crate) grid: f64,
    pub(crate) selected: Option<usize>,
    drag: Option<Drag>,
}

impl DesignerState {
    pub(crate) fn new(form: Form, grid: f64) -> DesignerState {
        DesignerState {
            form,
            grid,
            selected: None,
            drag: None,
        }
    }

    /// Replaces the whole model and clears the selection and any drag.
    pub(crate) fn replace_form(&mut self, form: Form) {
        self.form = form;
        self.selected = None;
        self.drag = None;
    }

    /// Resizes the form and anchors every control to the new surface.
    pub(crate) fn resize_form(&mut self, new_size: crate::model::Size) {
        apply_anchors(&mut self.form, new_size);
        self.drag = None;
    }

    /// Selects `index`, ignoring an out-of-range value.
    pub(crate) fn select(&mut self, index: Option<usize>) {
        let count = self.form.controls.len();
        self.selected = index.filter(|value| *value < count);
    }

    /// A client-pixel point as design units at the live `dpi`.
    fn point(&self, x: i32, y: i32, dpi: u32) -> DesignPoint {
        DesignPoint::new(px_to_dip(x, dpi), px_to_dip(y, dpi))
    }

    /// Handles a left press: grab a handle, select a control or clear.
    fn mouse_down(&mut self, x: i32, y: i32, dpi: u32) -> Option<DesignerEvent> {
        let point = self.point(x, y, dpi);
        if let Some(index) = self.selected
            && let Some(control) = self.form.controls.get(index)
            && let Some(handle) = interact::handle_at(control.bounds, point, HANDLE_REACH_DIP)
        {
            self.drag = Some(Drag::Resize {
                handle,
                origin: control.bounds,
            });
            return Some(DesignerEvent::SelectionChanged(Some(index)));
        }
        let hit = interact::hit_test(&self.form, point);
        self.selected = hit;
        self.drag = hit.map(|index| Drag::Move {
            start: point,
            origin: self.form.controls[index].bounds,
        });
        Some(DesignerEvent::SelectionChanged(hit))
    }

    /// Moves or resizes the control being dragged.
    fn mouse_move(&mut self, x: i32, y: i32, dpi: u32) -> Option<DesignerEvent> {
        let point = self.point(x, y, dpi);
        let index = self.selected?;
        match self.drag.as_ref()? {
            Drag::Move { start, origin } => {
                let delta = DesignPoint::new(point.x - start.x, point.y - start.y);
                let moved = interact::moved_bounds(*origin, delta, self.grid, self.form.size);
                self.form.controls[index].bounds = moved;
                Some(DesignerEvent::FormEdited)
            }
            Drag::Resize { handle, origin } => {
                let resized =
                    interact::resized_bounds(*origin, *handle, point, self.grid, MIN_SIZE_DIP);
                self.form.controls[index].bounds = resized;
                Some(DesignerEvent::FormEdited)
            }
        }
    }

    /// Ends a drag, snapping the moved control to the grid.
    fn mouse_up(&mut self) -> Option<DesignerEvent> {
        self.drag.take()?;
        if let Some(index) = self.selected
            && let Some(control) = self.form.controls.get_mut(index)
        {
            control.bounds = interact::snap_bounds(control.bounds, self.grid);
        }
        Some(DesignerEvent::FormEdited)
    }

    /// Abandons an in-flight drag without applying its last move.
    fn cancel_drag(&mut self) {
        self.drag = None;
    }

    /// Nudges or deletes the selected control.
    fn key(&mut self, key: Key) -> Option<DesignerEvent> {
        let index = self.selected?;
        let step = if self.grid > 0.0 { self.grid } else { 1.0 };
        let delta = match key {
            Key::LEFT => DesignPoint::new(-step, 0.0),
            Key::RIGHT => DesignPoint::new(step, 0.0),
            Key::UP => DesignPoint::new(0.0, -step),
            Key::DOWN => DesignPoint::new(0.0, step),
            Key::DELETE => {
                self.form.controls.remove(index);
                self.selected = None;
                self.drag = None;
                return Some(DesignerEvent::FormEdited);
            }
            _ => return None,
        };
        let control = self.form.controls.get(index)?;
        self.form.controls[index].bounds =
            interact::moved_bounds(control.bounds, delta, self.grid, self.form.size);
        Some(DesignerEvent::FormEdited)
    }
}

/// The owner-drawn widget behind a [`FormDesigner`](super::FormDesigner).
pub(crate) struct DesignerWidget {
    state: Rc<RefCell<DesignerState>>,
    /// Reads the window's current DPI through the live `Ui`, so a paint after a
    /// monitor move scales with the new DPI instead of a value cached when the
    /// widget was built.
    dpi: Rc<dyn Fn() -> u32>,
    /// The UI font, created once per DPI and reused across paints so a repaint
    /// allocates no GDI object.
    font: RefCell<Option<(u32, Rc<Font>)>>,
}

impl DesignerWidget {
    pub(crate) fn new(
        state: Rc<RefCell<DesignerState>>,
        dpi: Rc<dyn Fn() -> u32>,
    ) -> DesignerWidget {
        DesignerWidget {
            state,
            dpi,
            font: RefCell::new(None),
        }
    }

    fn font(&self, dpi: u32) -> Option<Rc<Font>> {
        if let Some((cached_dpi, font)) = self.font.borrow().as_ref()
            && *cached_dpi == dpi
        {
            return Some(Rc::clone(font));
        }
        let font = Rc::new(Font::system_ui(dpi).ok()?);
        *self.font.borrow_mut() = Some((dpi, Rc::clone(&font)));
        Some(font)
    }

    /// Runs one input through the state and, if it changed anything, repaints
    /// and raises the event.
    fn apply(&self, event: Option<DesignerEvent>, cx: &mut WidgetCx<DesignerEvent>) {
        if let Some(event) = event {
            cx.invalidate();
            cx.emit(event);
        }
    }
}

impl CustomWidget for DesignerWidget {
    type Event = DesignerEvent;

    fn paint(&self, canvas: &Canvas, bounds: Rect, theme: &Theme) {
        // `Canvas` carries no DPI, so ask the live `Ui` rather than a cached
        // value; a monitor move changes this between paints.
        let dpi = (self.dpi)();
        let state = self.state.borrow();
        let font = self.font(dpi);
        paint::paint(canvas, bounds, theme, &state, font.as_deref(), dpi);
    }

    fn preferred_size(&self, dpi: u32) -> Option<Size> {
        let size = size_to_rect(self.state.borrow().form.size, dpi);
        Some(Size::new(size.width(), size.height()))
    }

    fn wants_arrow_keys(&self) -> bool {
        // The arrows nudge the selection, so a hosting dialog must not use them
        // to move focus off the designer.
        true
    }

    fn input(&self, input: Input, cx: &mut WidgetCx<DesignerEvent>) {
        // Input coordinates are device pixels; convert with the context's live
        // DPI so hit-testing stays correct after a monitor move.
        let dpi = cx.dpi();
        let event = match input {
            Input::MouseDown {
                x,
                y,
                button: MouseButton::Left,
                ..
            } => {
                cx.focus();
                cx.capture();
                self.state.borrow_mut().mouse_down(x, y, dpi)
            }
            Input::MouseMove { x, y, .. } => self.state.borrow_mut().mouse_move(x, y, dpi),
            Input::MouseUp {
                button: MouseButton::Left,
                ..
            } => {
                cx.release_capture();
                self.state.borrow_mut().mouse_up()
            }
            // Another window took the capture, so the drag can never receive its
            // mouse-up; abandon it rather than leave the control stuck to the
            // pointer.
            Input::CaptureChanged => {
                self.state.borrow_mut().cancel_drag();
                None
            }
            Input::KeyDown { key, .. } => self.state.borrow_mut().key(key),
            _ => None,
        };
        self.apply(event, cx);
    }

    fn key(
        &self,
        key: Key,
        _modifiers: xui::Modifiers,
        cx: &mut WidgetCx<Self::Event>,
    ) -> KeyResult {
        // Reached only when the widget is hosted with the scroll host, which
        // sends navigation keys here before scrolling. Answer the arrows so they
        // nudge instead of scrolling.
        let event = self.state.borrow_mut().key(key);
        if event.is_some() {
            self.apply(event, cx);
            KeyResult::Handled
        } else {
            KeyResult::Ignored
        }
    }
}
