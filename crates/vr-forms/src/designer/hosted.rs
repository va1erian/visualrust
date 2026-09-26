//! Hosting one model control as a real xui widget.
//!
//! The portable widget layer covers most palette kinds. Kinds with no portable
//! widget yet are [`Hosted::Placeholder`]: the surface paints a labelled box
//! for them, but no child window is created, so they still hit-test and drag.

use xui::Rect;
use xui::xui_core::backend::WidgetId;
use xui::xui_core::widget::{
    Button, CheckBox, ComboBox, Edit, GroupBox, Label, ProgressBar, RadioGroup, Slider,
};

use crate::designer::DesignerError;
use crate::model::{Control, ControlKind};

/// The height of one radio option, mirroring `xui`'s own `RadioGroup` row.
pub(crate) const RADIO_ROW_DIP: f64 = 28.0;

/// A model control hosted on screen.
///
/// Every variant owns its node(s); dropping the value destroys them. A
/// [`Hosted::Placeholder`] owns nothing because the surface paints it.
pub enum Hosted<M: 'static> {
    Button(Button<M>),
    Edit(Edit<M>),
    Label(Label<M>),
    CheckBox(CheckBox<M>),
    ComboBox(ComboBox<M>),
    GroupBox(GroupBox<M>),
    ProgressBar(ProgressBar<M>),
    RadioGroup(RadioGroup<M>),
    Slider(Slider<M>),
    Placeholder,
}

/// Whether `kind` has a portable xui widget; the rest paint as placeholders.
pub fn is_portable(kind: &ControlKind) -> bool {
    matches!(
        kind,
        ControlKind::Button
            | ControlKind::Edit(_)
            | ControlKind::Label
            | ControlKind::CheckBox(_)
            | ControlKind::RadioGroup(_)
            | ControlKind::ComboBox(_)
            | ControlKind::GroupBox
            | ControlKind::ProgressBar(_)
            | ControlKind::Slider(_)
    )
}

impl<M: 'static> Hosted<M> {
    /// Creates the widget for `control` at `bounds` (device pixels).
    pub fn create(
        ui: &xui::xui_core::Ui<M>,
        control: &Control,
        bounds: Rect,
    ) -> Result<Hosted<M>, DesignerError> {
        let text = control.text.as_str();
        Ok(match &control.kind {
            ControlKind::Button => Hosted::Button(Button::new(ui, bounds, text)?),
            ControlKind::Label => Hosted::Label(Label::new(ui, bounds, text)?),
            ControlKind::GroupBox => Hosted::GroupBox(GroupBox::new(ui, bounds, text)?),
            ControlKind::Edit(_) => Hosted::Edit(Edit::new(ui, bounds, text)?),
            ControlKind::CheckBox(props) => {
                let widget = CheckBox::new(ui, bounds, text)?;
                widget.set_checked(props.checked);
                Hosted::CheckBox(widget)
            }
            ControlKind::ComboBox(props) => {
                let items = as_refs(&props.items);
                Hosted::ComboBox(ComboBox::new(ui, bounds, &items)?)
            }
            ControlKind::RadioGroup(props) => {
                let items = as_refs(&props.items);
                Hosted::RadioGroup(RadioGroup::new(ui, bounds, &items)?)
            }
            ControlKind::ProgressBar(props) => {
                let widget = ProgressBar::new(ui, bounds, props.max as i32)?;
                widget.set_value(props.value as i32);
                Hosted::ProgressBar(widget)
            }
            ControlKind::Slider(props) => {
                let widget = Slider::new(ui, bounds, props.min, props.max)?;
                widget.set_value(props.value);
                Hosted::Slider(widget)
            }
            _ => Hosted::Placeholder,
        })
    }

    /// The widget's node identities, in layout order.
    pub fn ids(&self) -> Vec<WidgetId> {
        match self {
            Hosted::Button(w) => vec![w.id()],
            Hosted::Edit(w) => vec![w.id()],
            Hosted::Label(w) => vec![w.id()],
            Hosted::CheckBox(w) => vec![w.id()],
            Hosted::ComboBox(w) => vec![w.id()],
            Hosted::GroupBox(w) => vec![w.id()],
            Hosted::ProgressBar(w) => vec![w.id()],
            Hosted::RadioGroup(w) => w.ids(),
            Hosted::Slider(w) => vec![w.id()],
            Hosted::Placeholder => Vec::new(),
        }
    }

    /// The per-node pixel rectangles for `bounds`, matching [`Hosted::ids`].
    ///
    /// A radio group is several single-row nodes, so each option gets its own
    /// rectangle; every other host is one node at `bounds`.
    pub fn layout(&self, bounds: Rect, dpi: u32) -> Vec<Rect> {
        match self {
            Hosted::RadioGroup(group) => {
                let row = crate::designer::convert::dip_to_px(RADIO_ROW_DIP, dpi);
                (0..group.ids().len())
                    .map(|index| {
                        let top = bounds.top + row * index as i32;
                        Rect::new(bounds.left, top, bounds.right, top + row)
                    })
                    .collect()
            }
            Hosted::Placeholder => Vec::new(),
            _ => vec![bounds],
        }
    }
}

/// Borrows a slice of owned strings as `&str` for the widget constructors.
fn as_refs(items: &[String]) -> Vec<&str> {
    items.iter().map(String::as_str).collect()
}
