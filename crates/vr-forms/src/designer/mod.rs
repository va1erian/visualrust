//! The live designer surface.
//!
//! A [`DesignerSurface`] puts a [`Form`] model on screen as real xui widgets
//! and lets the user manipulate it directly. The model is the state: every
//! edit (select, move, resize, nudge, delete) mutates the [`Form`]'s dip
//! bounds and raises a [`SurfaceEvent`] so the hosting app can persist the
//! form or refresh its own views.
//!
//! Input is handled by the designer, not the widgets: the surface turns on
//! `Ui::set_design_mode(true)` and replaces each hosted widget's event mapper
//! with its own, so a click selects or drags instead of pressing a button.
//! The surface itself is a painted [`NodeKind::Custom`] node created first, so
//! it draws the grid and adorners underneath the widgets.

mod convert;
mod hosted;
mod interact;
mod paint;

pub use hosted::is_portable;
pub use interact::{DesignPoint, Handle};

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use xui::Rect;
use xui::xui_core::backend::{Event, NodeKind, NodeSpec, WidgetId};
use xui::xui_core::message::{Key, MouseButton};
use xui::xui_core::{BackendError, Painter, Ui};

use crate::error::ValidationError;
use crate::model::{Bounds, Dip as ModelDip, Form};
use hosted::Hosted;

/// The smallest a resize may leave a control, in design units.
const MIN_SIZE_DIP: f64 = 8.0;

/// How close, in design units, a pointer must be to a handle to grab it.
const HANDLE_REACH_DIP: f64 = 6.0;

/// Why a designer surface could not be built.
#[derive(Debug, thiserror::Error)]
pub enum DesignerError {
    #[error("could not create a designer widget: {0}")]
    Widget(#[from] BackendError),
    #[error("form is not valid: {0}")]
    Invalid(#[from] ValidationError),
}

/// What changed on the surface, mapped to the app's message by
/// [`DesignerSurface::on_change`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceEvent {
    /// The selected control changed (`None` is an empty selection).
    SelectionChanged(Option<usize>),
    /// The form's controls or their bounds changed.
    FormEdited,
}

/// Maps a surface event to the app's message.
type ChangeMapper<M> = Rc<RefCell<Option<Box<dyn Fn(SurfaceEvent) -> Option<M>>>>>;

/// The device-pixel origin a node's events are local to; shared so [`relayout`]
/// keeps it current as the node moves.
type NodeOffset = Rc<Cell<(i32, i32)>>;

/// A drag in progress.
enum Drag {
    Move { start: DesignPoint, origin: Bounds },
    Resize { handle: Handle, origin: Bounds },
}

/// The mutable state shared by the painter and the input handlers.
struct DesignerState<M: 'static> {
    form: Form,
    grid: f64,
    dpi: u32,
    selected: Option<usize>,
    drag: Option<Drag>,
    /// Hosted widgets per control; owning them keeps the nodes alive.
    widgets: Vec<Hosted<M>>,
    /// Per control, whether it paints as a placeholder rather than a widget.
    placeholders: Vec<bool>,
    /// Per hosted node, the device-pixel origin its events are local to, kept
    /// current by [`relayout`] so a click maps back to form space even as the
    /// node moves under the pointer.
    node_cells: Vec<Vec<NodeOffset>>,
    surface: WidgetId,
    min_size: f64,
}

/// A form model hosted as live, directly manipulable widgets.
pub struct DesignerSurface<M: 'static> {
    ui: Ui<M>,
    state: Rc<RefCell<DesignerState<M>>>,
    on_change: ChangeMapper<M>,
}

impl<M: 'static> DesignerSurface<M> {
    /// Hosts `form` on the window behind `ui`, snapping edits to `grid`.
    pub fn new(
        ui: &Ui<M>,
        form: Form,
        grid: ModelDip,
    ) -> Result<DesignerSurface<M>, DesignerError> {
        form.validate()?;
        // The designer owns input; the widgets must not act on it themselves.
        ui.set_design_mode(true);
        let dpi = ui.dpi();
        let surface = ui.create_node(&NodeSpec::new(
            NodeKind::Custom,
            convert::size_to_rect(form.size, dpi),
        ))?;
        let state = Rc::new(RefCell::new(DesignerState {
            form,
            grid: grid.get(),
            dpi,
            selected: None,
            drag: None,
            widgets: Vec::new(),
            placeholders: Vec::new(),
            node_cells: Vec::new(),
            surface,
            min_size: MIN_SIZE_DIP,
        }));

        {
            let state = Rc::clone(&state);
            let theme = ui.theme_handle();
            let painter: Painter = Rc::new(move |canvas| {
                let state = state.borrow();
                paint::paint_surface(
                    canvas,
                    &theme.get(),
                    &state.form,
                    &state.placeholders,
                    state.selected,
                    state.grid,
                    state.dpi,
                );
            });
            ui.set_painter(surface, painter);
        }

        let on_change: ChangeMapper<M> = Rc::new(RefCell::new(None));
        // The surface sits at the form origin, so its own events need no shift.
        install_handler(ui, surface, &state, &on_change, Rc::new(Cell::new((0, 0))));

        let mut surface = DesignerSurface {
            ui: ui.clone(),
            state,
            on_change,
        };
        surface.rebuild()?;
        Ok(surface)
    }

    /// The current form model.
    pub fn form(&self) -> Form {
        self.state.borrow().form.clone()
    }

    /// Replaces the whole form, rebuilding every hosted widget.
    pub fn set_form(&mut self, form: Form) -> Result<(), DesignerError> {
        form.validate()?;
        {
            let mut state = self.state.borrow_mut();
            state.form = form;
            state.selected = None;
            state.drag = None;
            // Dropping the old widgets destroys their nodes.
            state.widgets.clear();
            state.node_cells.clear();
        }
        self.rebuild()
    }

    /// The grid step, in design units.
    pub fn grid(&self) -> ModelDip {
        ModelDip::new(self.state.borrow().grid)
    }

    /// Changes the grid step and repaints.
    pub fn set_grid(&mut self, grid: ModelDip) {
        let surface = {
            let mut state = self.state.borrow_mut();
            state.grid = grid.get();
            state.surface
        };
        self.ui.invalidate(surface);
    }

    /// The selected control's index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected
    }

    /// Selects `index` (or clears the selection) without a click.
    pub fn select(&mut self, index: Option<usize>) {
        let surface = {
            let mut state = self.state.borrow_mut();
            let count = state.form.controls.len();
            state.selected = index.filter(|value| *value < count);
            state.surface
        };
        self.ui.invalidate(surface);
    }

    /// The surface node's identity, for callers that want to move or capture it.
    pub fn surface_id(&self) -> WidgetId {
        self.state.borrow().surface
    }

    /// Maps surface changes to the app's message.
    pub fn on_change(&mut self, mapper: impl Fn(SurfaceEvent) -> Option<M> + 'static) {
        *self.on_change.borrow_mut() = Some(Box::new(mapper));
    }

    /// Builds a widget for every control and installs the designer's input
    /// handler on each.
    fn rebuild(&mut self) -> Result<(), DesignerError> {
        let controls = self.state.borrow().form.controls.clone();
        let dpi = self.state.borrow().dpi;
        let mut widgets = Vec::with_capacity(controls.len());
        let mut placeholders = Vec::with_capacity(controls.len());
        let mut node_cells = Vec::with_capacity(controls.len());
        for control in &controls {
            let bounds = convert::bounds_to_rect(control.bounds, dpi);
            // A kind the backend cannot host — the native `Edit` on a machine
            // with no interactive desktop, say — degrades to a painted
            // placeholder rather than failing the whole surface. The model and
            // its manipulation still work; only the live widget is missing.
            let widget = Hosted::create(&self.ui, control, bounds).unwrap_or(Hosted::Placeholder);
            placeholders.push(matches!(widget, Hosted::Placeholder));
            node_cells.push(
                widget
                    .ids()
                    .iter()
                    .map(|_| Rc::new(Cell::new((0, 0))))
                    .collect(),
            );
            widgets.push(widget);
        }
        {
            let mut state = self.state.borrow_mut();
            state.widgets = widgets;
            state.placeholders = placeholders;
            state.node_cells = node_cells;
        }
        // Position first: the handlers read the offset each node's events are
        // local to, so `relayout` must have filled it in.
        relayout(&self.state, &self.ui);
        let nodes: Vec<(WidgetId, NodeOffset)> = {
            let state = self.state.borrow();
            let mut nodes = Vec::new();
            for (index, widget) in state.widgets.iter().enumerate() {
                for (row, id) in widget.ids().into_iter().enumerate() {
                    if let Some(cell) = state.node_cells.get(index).and_then(|rows| rows.get(row)) {
                        nodes.push((id, Rc::clone(cell)));
                    }
                }
            }
            nodes
        };
        for (id, offset) in nodes {
            install_handler(&self.ui, id, &self.state, &self.on_change, offset);
        }
        Ok(())
    }
}

/// Registers the designer's input handler in place of a widget's own.
fn install_handler<M: 'static>(
    ui: &Ui<M>,
    id: WidgetId,
    state: &Rc<RefCell<DesignerState<M>>>,
    on_change: &ChangeMapper<M>,
    offset: NodeOffset,
) {
    let state = Rc::clone(state);
    let on_change = Rc::clone(on_change);
    let inner = ui.clone();
    ui.register_events(id, move |event| {
        // A painted widget's events carry coordinates local to its own window,
        // so a click on a hosted control is moved into form space before it is
        // hit-tested. The offset tracks the node as a drag moves it.
        let (dx, dy) = offset.get();
        let change = handle_event(&state, &inner, event, (dx, dy));
        change.and_then(|change| {
            on_change
                .borrow()
                .as_ref()
                .and_then(|mapper| mapper(change))
        })
    });
}

/// Applies one event to the model, returning what changed.
fn handle_event<M: 'static>(
    state: &Rc<RefCell<DesignerState<M>>>,
    ui: &Ui<M>,
    event: &Event,
    offset: (i32, i32),
) -> Option<SurfaceEvent> {
    let change = {
        let mut state = state.borrow_mut();
        on_input(&mut state, event, offset)
    };
    if change.is_some() {
        relayout(state, ui);
    }
    change
}

/// The pure input-to-model step, kept separate so the borrow is scoped.
fn on_input<M: 'static>(
    state: &mut DesignerState<M>,
    event: &Event,
    offset: (i32, i32),
) -> Option<SurfaceEvent> {
    match *event {
        Event::MouseDown {
            x,
            y,
            button: MouseButton::Left,
            ..
        } => {
            let point = form_point(x, y, offset, state.dpi);
            if let Some(index) = state.selected
                && let Some(control) = state.form.controls.get(index)
                && let Some(handle) = interact::handle_at(control.bounds, point, HANDLE_REACH_DIP)
            {
                state.drag = Some(Drag::Resize {
                    handle,
                    origin: control.bounds,
                });
                return Some(SurfaceEvent::SelectionChanged(Some(index)));
            }
            let hit = interact::hit_test(&state.form, point);
            state.selected = hit;
            state.drag = hit.map(|index| Drag::Move {
                start: point,
                origin: state.form.controls[index].bounds,
            });
            Some(SurfaceEvent::SelectionChanged(hit))
        }
        Event::MouseMove { x, y, .. } => {
            let point = form_point(x, y, offset, state.dpi);
            let index = state.selected?;
            match state.drag {
                Some(Drag::Move { start, origin }) => {
                    let delta = DesignPoint::new(point.x - start.x, point.y - start.y);
                    let moved = interact::moved_bounds(origin, delta, state.grid, state.form.size);
                    state.form.controls[index].bounds = moved;
                    Some(SurfaceEvent::FormEdited)
                }
                Some(Drag::Resize { handle, origin }) => {
                    let resized =
                        interact::resized_bounds(origin, handle, point, state.grid, state.min_size);
                    state.form.controls[index].bounds = resized;
                    Some(SurfaceEvent::FormEdited)
                }
                None => None,
            }
        }
        Event::MouseUp {
            button: MouseButton::Left,
            ..
        } => {
            state.drag.take()?;
            if let Some(index) = state.selected
                && let Some(control) = state.form.controls.get_mut(index)
            {
                control.bounds = interact::snap_bounds(control.bounds, state.grid);
            }
            Some(SurfaceEvent::FormEdited)
        }
        Event::KeyDown { key, .. } => key_input(state, key),
        _ => None,
    }
}

/// Nudges or deletes the selected control.
fn key_input<M: 'static>(state: &mut DesignerState<M>, key: Key) -> Option<SurfaceEvent> {
    let index = state.selected?;
    let step = if state.grid > 0.0 { state.grid } else { 1.0 };
    let delta = match key {
        Key::LEFT => DesignPoint::new(-step, 0.0),
        Key::RIGHT => DesignPoint::new(step, 0.0),
        Key::UP => DesignPoint::new(0.0, -step),
        Key::DOWN => DesignPoint::new(0.0, step),
        Key::DELETE => {
            state.form.controls.remove(index);
            if index < state.widgets.len() {
                // Dropping the widget destroys its node.
                state.widgets.remove(index);
                state.placeholders.remove(index);
                state.node_cells.remove(index);
            }
            state.selected = None;
            state.drag = None;
            return Some(SurfaceEvent::FormEdited);
        }
        _ => return None,
    };
    let control = state.form.controls.get(index)?;
    let moved = interact::moved_bounds(control.bounds, delta, state.grid, state.form.size);
    state.form.controls[index].bounds = moved;
    Some(SurfaceEvent::FormEdited)
}

/// Converts a device-pixel point to design units.
fn point_at(x: i32, y: i32, dpi: u32) -> DesignPoint {
    DesignPoint::new(convert::px_to_dip(x, dpi), convert::px_to_dip(y, dpi))
}

/// A point in a node's local client pixels, moved into form space by the node's
/// device-pixel origin, then into design units.
fn form_point(x: i32, y: i32, offset: (i32, i32), dpi: u32) -> DesignPoint {
    point_at(x + offset.0, y + offset.1, dpi)
}

/// Moves every hosted node to its control's current bounds and repaints.
fn relayout<M: 'static>(state: &Rc<RefCell<DesignerState<M>>>, ui: &Ui<M>) {
    let state = state.borrow();
    let mut moves: Vec<(WidgetId, Rect)> = Vec::new();
    for (index, control) in state.form.controls.iter().enumerate() {
        let Some(widget) = state.widgets.get(index) else {
            continue;
        };
        let bounds = convert::bounds_to_rect(control.bounds, state.dpi);
        let rects = widget.layout(bounds, state.dpi);
        for (row, (id, rect)) in widget.ids().into_iter().zip(rects).enumerate() {
            // Keep the offset the node's events are local to in step with the
            // move, so a drag stays correct as the node follows the pointer.
            if let Some(cell) = state.node_cells.get(index).and_then(|rows| rows.get(row)) {
                cell.set((rect.left, rect.top));
            }
            moves.push((id, rect));
        }
    }
    ui.apply_moves(&moves);
    ui.invalidate(state.surface);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_local_point_is_moved_into_form_space() {
        // At 96 dpi one pixel is one design unit, so the node's origin is a
        // direct offset.
        assert_eq!(
            form_point(5, 7, (100, 50), 96),
            DesignPoint::new(105.0, 57.0)
        );
    }

    #[test]
    fn the_surface_offset_leaves_the_point_unchanged() {
        assert_eq!(form_point(5, 7, (0, 0), 96), DesignPoint::new(5.0, 7.0));
    }
}
