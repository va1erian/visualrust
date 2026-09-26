//! The form designer on the `xui` (xui_win32) widget layer.
//!
//! A [`FormDesigner`] hosts a single owner-drawn
//! [`Custom`](xui::Custom) widget that paints the form page, the grid, a themed
//! proxy box per control and the selection adorners, and turns mouse and
//! keyboard input into edits. It is built on the native widget layer precisely
//! so it can be dropped into the IDE shell's `xui_win32` app; the earlier
//! surface built on `xui_core::Ui` could not be constructed from that window.
//!
//! Editing maths lives in the pure [`interact`] module, the model inspection
//! and mutation in [`props`], painting in [`paint`] and the input decoding in
//! [`widget`]. The designer's state is interior-mutable (`Rc<RefCell<..>>`)
//! because [`CustomWidget`](xui::CustomWidget) methods take `&self`.
//!
//! ```no_run
//! use vr_forms::{ControlKind, Dip, Form, design::FormDesigner};
//!
//! fn build<M: 'static>(
//!     ui: &mut xui::Ui<M>,
//!     form: Form,
//! ) -> Result<(), vr_forms::design::DesignerError> {
//!     let mut designer = FormDesigner::new(ui, form, Dip::new(8.0))?;
//!     let index = designer.add_control(ControlKind::Button);
//!     designer.select(Some(index));
//!     let _ = designer;
//!     Ok(())
//! }
//! ```

mod interact;
mod paint;
mod props;
mod units;
mod widget;

pub use interact::{DesignPoint, Handle};
pub use props::PropertyValue;

use std::cell::RefCell;
use std::rc::Rc;

use xui::{AsControl, Control, Custom, Ui};

use crate::error::ValidationError;
use crate::model::{ControlKind, Dip, Form, Size};

use widget::{DesignerState, DesignerWidget};

/// Why a designer could not be built.
#[derive(Debug, thiserror::Error)]
pub enum DesignerError {
    /// The `Custom` widget (and so its child window) could not be created,
    /// which is expected on a session without an interactive desktop.
    #[error("could not create a designer widget: {0}")]
    Widget(#[from] xui::Error),
    /// The form handed in does not satisfy the model's structural rules.
    #[error("form is not valid: {0}")]
    Invalid(#[from] ValidationError),
}

/// What changed on the designer, mapped to the app's message by
/// [`FormDesigner::on_change`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignerEvent {
    /// The selected control changed (`None` is an empty selection).
    SelectionChanged(Option<usize>),
    /// The form's controls or their bounds changed.
    FormEdited,
}

/// Maps a designer event to the app's message.
type ChangeMapper<M> = Rc<RefCell<Option<Rc<dyn Fn(DesignerEvent) -> Option<M>>>>>;

/// A form model hosted as a single, directly manipulable owner-drawn widget.
///
/// Hold the value in the app's state so its `Custom` (and the child window it
/// owns) stays alive. As an [`AsControl`] it can be placed by a layout like any
/// other widget.
pub struct FormDesigner<M: 'static> {
    custom: Custom<DesignerWidget, M>,
    ui: Ui<M>,
    state: Rc<RefCell<DesignerState>>,
    on_change: ChangeMapper<M>,
}

impl<M: 'static> FormDesigner<M> {
    /// Hosts `form` inside `ui`, snapping edits to `grid`.
    pub fn new(ui: &mut Ui<M>, form: Form, grid: Dip) -> Result<FormDesigner<M>, DesignerError> {
        form.validate()?;
        let state = Rc::new(RefCell::new(DesignerState::new(form, grid.get())));
        // The widget reads the DPI through this probe on every paint, so a
        // monitor move is picked up without a second `FormDesigner::new`.
        let dpi_probe: Rc<dyn Fn() -> u32> = {
            let ui = ui.clone();
            Rc::new(move || ui.dpi())
        };
        let widget = DesignerWidget::new(Rc::clone(&state), dpi_probe);
        let on_change: ChangeMapper<M> = Rc::new(RefCell::new(None));
        let mapper = Rc::clone(&on_change);
        let custom = Custom::new(ui, widget)?.on_event(move |event| {
            let mapper = mapper.borrow();
            mapper.as_ref().and_then(|map| map(event))
        });
        Ok(FormDesigner {
            custom,
            ui: ui.clone(),
            state,
            on_change,
        })
    }

    /// The current form model.
    pub fn form(&self) -> Form {
        self.state.borrow().form.clone()
    }

    /// Replaces the whole form, clearing the selection; an invalid form is left
    /// out rather than put on screen. Use [`FormDesigner::try_set_form`] to see
    /// why a form was rejected.
    pub fn set_form(&mut self, form: Form) {
        let _ = self.try_set_form(form);
    }

    /// Like [`FormDesigner::set_form`], but reports an invalid form.
    pub fn try_set_form(&mut self, form: Form) -> Result<(), DesignerError> {
        form.validate()?;
        self.state.borrow_mut().replace_form(form);
        self.custom.invalidate();
        Ok(())
    }

    /// The grid step, in design units.
    pub fn grid(&self) -> Dip {
        Dip::new(self.state.borrow().grid)
    }

    /// Changes the grid step and repaints.
    pub fn set_grid(&mut self, grid: Dip) {
        self.state.borrow_mut().grid = grid.get();
        self.custom.invalidate();
    }

    /// Anchors every control for a form resize to `new_size`, adopts the new
    /// size and repaints.
    ///
    /// The hosting app calls this when its window's client area changes, so the
    /// design preview matches what `apply_anchors` will do at run time.
    pub fn resize_form(&mut self, new_size: Size) {
        self.state.borrow_mut().resize_form(new_size);
        self.custom.invalidate();
        self.emit(DesignerEvent::FormEdited);
    }

    /// Selects `index` (or clears the selection) without a click.
    pub fn select(&self, index: Option<usize>) {
        self.state.borrow_mut().select(index);
        self.custom.invalidate();
    }

    /// The selected control's index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected
    }

    /// Appends a control of `kind` at a default position, selects it and
    /// returns its index. The generated name is unique within the form.
    pub fn add_control(&mut self, kind: ControlKind) -> usize {
        let index = {
            let mut state = self.state.borrow_mut();
            let grid = state.grid;
            let index = props::add(&mut state.form, kind, grid);
            state.selected = Some(index);
            index
        };
        self.custom.invalidate();
        self.emit(DesignerEvent::SelectionChanged(Some(index)));
        self.emit(DesignerEvent::FormEdited);
        index
    }

    /// Reads property `name` from the selected control, if any.
    pub fn property(&self, name: &str) -> Option<PropertyValue> {
        let state = self.state.borrow();
        let index = state.selected?;
        props::get(&state.form, index, name)
    }

    /// Writes property `name` on the selected control, returning whether the
    /// write applied. Edits that would leave the form invalid are rejected.
    pub fn set_property(&mut self, name: &str, value: PropertyValue) -> bool {
        let applied = {
            let mut state = self.state.borrow_mut();
            match state.selected {
                Some(index) => props::set(&mut state.form, index, name, value),
                None => false,
            }
        };
        if applied {
            self.custom.invalidate();
            self.emit(DesignerEvent::FormEdited);
        }
        applied
    }

    /// Maps designer changes to the app's message.
    pub fn on_change(&mut self, mapper: impl Fn(DesignerEvent) -> Option<M> + 'static) {
        *self.on_change.borrow_mut() = Some(Rc::new(mapper));
    }

    /// Maps `event` through the change mapper and enqueues the result, for
    /// edits the app makes through this API rather than through input.
    fn emit(&self, event: DesignerEvent) {
        if let Some(mapper) = self.on_change.borrow().as_ref()
            && let Some(msg) = mapper(event)
        {
            self.ui.emit(msg);
        }
    }
}

impl<M: 'static> AsControl for FormDesigner<M> {
    fn control(&self) -> &Control {
        self.custom.control()
    }
}
