//! Materialising a [`WidgetPlan`] into a live xui control.
//!
//! Each plan becomes exactly one variant of [`Live`]; the shared
//! [`ControlExt`]/[`LayoutExt`] behaviour is written once through the
//! [`with_widget`](with_widget!) macro. A family xui has no direct child for
//! degrades to a labelled placeholder so a generated form always opens.

use std::rc::Rc;

use xui::controls::slider::Slider;
use xui::{
    Button, CheckBox, Color, ColorPicker, ControlExt, Dip, Edit, FlowText, GroupBox, HasText,
    Label, LayoutExt, LayoutItem, Panel, ProgressBar, Rect, Run, ScrollView, StatusBar, Ui,
};

use super::UiEvent;
use super::host::{HostError, classify};
use super::plan::{EventKind, WidgetKind, WidgetPlan};

/// Environment variable that logs each widget family degraded to a placeholder.
const DEBUG_ENV: &str = "VR_DYON_UI_DEBUG";
/// The slider range used when the form did not set one.
const DEFAULT_SLIDER: (f64, f64) = (0.0, 100.0);
/// Appended to a placeholder caption so a degraded control is identifiable.
const PLACEHOLDER_SUFFIX: &str = " [placeholder]";

/// What a handler receives for an event, before it is typed into a Dyon value.
pub(super) enum EventPayload {
    None,
    Text(String),
    Checked(bool),
    Value(f64),
}

/// Materialises one widget and wires its events to [`UiEvent`].
pub(super) fn build_widget(
    ui: &mut Ui<UiEvent>,
    plan: &WidgetPlan,
    dpi: u32,
) -> Result<Live, HostError> {
    let id = plan.id;
    let click = move || {
        Some(UiEvent {
            handle: id,
            event: EventKind::Click,
        })
    };
    let live = match plan.kind {
        WidgetKind::Label => Live::Label(
            Label::new(ui, device_rect(plan.bounds, dpi), &plan.text).map_err(classify)?,
        ),
        WidgetKind::Button => Live::Button(
            Button::new(ui, &plan.text)
                .map_err(classify)?
                .on_click(click),
        ),
        WidgetKind::Edit => {
            let edit = if plan.setters.password == Some(true) {
                Edit::password(ui)
            } else if plan.setters.multiline == Some(true) {
                Edit::multi_line(ui)
            } else {
                Edit::single_line(ui)
            }
            .map_err(classify)?;
            let edit = edit.on_change(move |_| {
                Some(UiEvent {
                    handle: id,
                    event: EventKind::Change,
                })
            });
            let edit = match &plan.setters.placeholder {
                Some(text) => edit.cue(text),
                None => edit,
            };
            Live::Edit(edit)
        }
        WidgetKind::CheckBox => Live::CheckBox(
            CheckBox::new(ui, &plan.text)
                .map_err(classify)?
                .on_toggle(move |_| {
                    Some(UiEvent {
                        handle: id,
                        event: EventKind::Change,
                    })
                }),
        ),
        WidgetKind::GroupBox => Live::GroupBox(GroupBox::new(ui, &plan.text).map_err(classify)?),
        WidgetKind::Panel => Live::Panel(Panel::new(ui).map_err(classify)?),
        WidgetKind::ProgressBar => {
            let bar = ProgressBar::new(ui).map_err(classify)?;
            if let Some((min, max)) = plan.setters.range {
                bar.set_range(min.round() as i32..=max.round() as i32);
            }
            if let Some(value) = plan.setters.value {
                bar.set_value(value.round() as i32);
            }
            Live::ProgressBar(bar)
        }
        WidgetKind::Slider => {
            let (min, max) = plan.setters.range.unwrap_or(DEFAULT_SLIDER);
            let mut slider = Slider::new(ui, min..=max).map_err(classify)?;
            if plan.setters.orientation.as_deref() == Some("vertical") {
                slider = slider.vertical();
            }
            if let Some(value) = plan.setters.value {
                slider.set_value(value);
            }
            Live::Slider(slider.on_change(move |_| {
                Some(UiEvent {
                    handle: id,
                    event: EventKind::Change,
                })
            }))
        }
        WidgetKind::ColorPicker => {
            let initial = plan
                .setters
                .color
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(Color::rgb(255, 255, 255));
            Live::ColorPicker(ColorPicker::new(ui, initial).map_err(classify)?.on_change(
                move |_| {
                    Some(UiEvent {
                        handle: id,
                        event: EventKind::Change,
                    })
                },
            ))
        }
        WidgetKind::FlowText => {
            let mut flow = FlowText::new(ui).map_err(classify)?;
            let on_click = Rc::new(click);
            for line in plan.text.split('\n') {
                let mapper = Rc::clone(&on_click);
                flow = flow.run(Run::normal(line).on_click(move || mapper()));
            }
            Live::FlowText(flow)
        }
        WidgetKind::ScrollView => Live::ScrollView(ScrollView::new(ui).map_err(classify)?),
        WidgetKind::StatusBar => {
            let bar = StatusBar::new(ui).map_err(classify)?;
            if let Some(items) = &plan.setters.items
                && let Some(first) = items.first()
            {
                bar.set_text(0, first);
            }
            Live::StatusBar(bar)
        }
        // xui has no direct child control for the model-driven families yet, so
        // a generated form still opens with a labelled placeholder.
        degraded => Live::Placeholder(placeholder(
            ui,
            plan,
            device_rect(plan.bounds, dpi),
            degraded,
        )?),
    };

    live.set_bounds(device_rect(plan.bounds, dpi));
    if !plan.enabled {
        live.set_enabled(false);
    }
    if !plan.visible {
        live.set_visible(false);
    }
    if let Some(tooltip) = &plan.tooltip {
        live.set_tooltip(tooltip);
    }
    if !matches!(live, Live::FlowText(_)) {
        live.set_text(&plan.text);
    }
    if let Live::CheckBox(box_) = &live
        && let Some(checked) = plan.setters.checked
    {
        box_.set_checked(checked);
    }
    Ok(live)
}

/// A labelled placeholder for a family without a child control.
fn placeholder(
    ui: &mut Ui<UiEvent>,
    plan: &WidgetPlan,
    bounds: Rect,
    kind: WidgetKind,
) -> Result<Label, HostError> {
    if std::env::var_os(DEBUG_ENV).is_some() {
        eprintln!("vr-dyon: {kind:?} has no xui child control; using a Label placeholder");
    }
    let caption = format!("{}{PLACEHOLDER_SUFFIX}", plan.text);
    Label::new(ui, bounds, &caption).map_err(classify)
}

/// The live widget, one variant per materialised xui control.
pub(super) enum Live {
    Label(Label),
    Button(Button<UiEvent>),
    Edit(Edit<UiEvent>),
    CheckBox(CheckBox<UiEvent>),
    GroupBox(GroupBox),
    Panel(Panel),
    ProgressBar(ProgressBar),
    Slider(Slider<UiEvent>),
    ColorPicker(ColorPicker<UiEvent>),
    FlowText(FlowText<UiEvent>),
    ScrollView(ScrollView),
    StatusBar(StatusBar<UiEvent>),
    Placeholder(Label),
}

/// Invokes `$body` with the concrete widget binding for every variant, so the
/// shared [`ControlExt`]/[`LayoutExt`] behaviour is written once.
macro_rules! with_widget {
    ($live:expr, $widget:ident => $body:expr) => {
        match $live {
            Live::Label($widget) => $body,
            Live::Button($widget) => $body,
            Live::Edit($widget) => $body,
            Live::CheckBox($widget) => $body,
            Live::GroupBox($widget) => $body,
            Live::Panel($widget) => $body,
            Live::ProgressBar($widget) => $body,
            Live::Slider($widget) => $body,
            Live::ColorPicker($widget) => $body,
            Live::FlowText($widget) => $body,
            Live::ScrollView($widget) => $body,
            Live::StatusBar($widget) => $body,
            Live::Placeholder($widget) => $body,
        }
    };
}

impl Live {
    pub(super) fn set_bounds(&self, bounds: Rect) {
        with_widget!(self, widget => widget.set_bounds(bounds))
    }

    pub(super) fn set_enabled(&self, enabled: bool) {
        with_widget!(self, widget => widget.set_enabled(enabled))
    }

    pub(super) fn set_visible(&self, visible: bool) {
        with_widget!(self, widget => widget.set_visible(visible))
    }

    pub(super) fn set_tooltip(&self, text: &str) {
        with_widget!(self, widget => widget.set_tooltip(text))
    }

    /// The widget as a layout item that follows its anchor in a free layout.
    pub(super) fn anchor_item(&self, anchor: xui::Anchor) -> LayoutItem {
        with_widget!(self, widget => widget.anchor(anchor))
    }

    /// Replaces the visible text on the families that carry one.
    pub(super) fn set_text(&self, text: &str) {
        match self {
            Live::Label(widget) => widget.set_text(text),
            Live::Button(widget) => widget.set_text(text),
            Live::Edit(widget) => widget.set_text(text),
            Live::CheckBox(widget) => widget.set_text(text),
            Live::GroupBox(widget) => widget.set_text(text),
            Live::Placeholder(widget) => widget.set_text(text),
            Live::Panel(_)
            | Live::ProgressBar(_)
            | Live::Slider(_)
            | Live::ColorPicker(_)
            | Live::FlowText(_)
            | Live::ScrollView(_)
            | Live::StatusBar(_) => {}
        }
    }

    /// The typed payload an event carries, where a family has a meaningful one.
    pub(super) fn payload(&self, _event: EventKind) -> EventPayload {
        match self {
            Live::Label(widget) | Live::Placeholder(widget) => EventPayload::Text(widget.text()),
            Live::Button(widget) => EventPayload::Text(widget.text()),
            Live::Edit(widget) => EventPayload::Text(widget.text()),
            Live::GroupBox(widget) => EventPayload::Text(widget.text()),
            Live::CheckBox(widget) => EventPayload::Checked(widget.is_checked()),
            Live::Slider(widget) => EventPayload::Value(widget.current_value()),
            _ => EventPayload::None,
        }
    }
}

/// Converts design bounds to device pixels at `dpi`.
fn device_rect(bounds: Rect, dpi: u32) -> Rect {
    let left = Dip::new(bounds.left as f32).to_px(dpi).value();
    let top = Dip::new(bounds.top as f32).to_px(dpi).value();
    let right = Dip::new(bounds.right as f32).to_px(dpi).value();
    let bottom = Dip::new(bounds.bottom as f32).to_px(dpi).value();
    Rect::new(left, top, right, bottom)
}

/// Parses `#rrggbb` into a colour, rejecting anything else rather than guessing.
fn parse_color(text: &str) -> Option<Color> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok().map(Color::hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rrggbb_only() {
        assert_eq!(parse_color("#a1b2c3"), Some(Color::rgb(0xa1, 0xb2, 0xc3)));
        assert_eq!(parse_color("a1b2c3"), None);
        assert_eq!(parse_color("#xyz"), None);
    }
}
