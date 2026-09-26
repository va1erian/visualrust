//! The native `ui_*` functions Dyon calls, and their registration signatures.
//!
//! Every binding is a thin translation between Dyon variables and the
//! [`plan`](super::plan) builder; no xui type is named here, so the signatures
//! stay checkable by the Dyon lifetime checker and the module can be registered
//! before a window exists.

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use super::plan::{self, WidgetKind};

/// Registers the whole `ui_*` vocabulary into `module`.
///
/// Must run before source is loaded so the lifetime checker sees the signatures.
pub(crate) fn register(module: &mut Module) {
    let window = ad_hoc("Window");
    let free = ad_hoc("Free");
    let widget = ad_hoc("Widget");
    let text_array = Type::Array(Box::new(Type::Str));

    module.add_str(
        "ui_window",
        ui_window,
        Dfn::nl(vec![Type::Str, Type::F64, Type::F64], window.clone()),
    );
    module.add_str(
        "ui_free",
        ui_free,
        Dfn::nl(vec![Type::F64, Type::F64], free.clone()),
    );
    module.add_str(
        "ui_add",
        ui_add,
        Dfn::nl(vec![free.clone(), widget.clone()], Type::Void),
    );
    module.add_str("ui_run", ui_run, Dfn::nl(vec![window, free], Type::Void));

    let ctor_sig = Dfn::nl(
        vec![Type::Str, Type::F64, Type::F64, Type::F64, Type::F64],
        widget.clone(),
    );
    module.add_str("ui_button", ui_button, ctor_sig.clone());
    module.add_str("ui_edit", ui_edit, ctor_sig.clone());
    module.add_str("ui_label", ui_label, ctor_sig.clone());
    module.add_str("ui_check_box", ui_check_box, ctor_sig.clone());
    module.add_str("ui_radio_group", ui_radio_group, ctor_sig.clone());
    module.add_str("ui_combo_box", ui_combo_box, ctor_sig.clone());
    module.add_str("ui_list_view", ui_list_view, ctor_sig.clone());
    module.add_str("ui_tree_view", ui_tree_view, ctor_sig.clone());
    module.add_str("ui_group_box", ui_group_box, ctor_sig.clone());
    module.add_str("ui_panel", ui_panel, ctor_sig.clone());
    module.add_str("ui_scroll_view", ui_scroll_view, ctor_sig.clone());
    module.add_str("ui_tabs", ui_tabs, ctor_sig.clone());
    module.add_str("ui_menu", ui_menu, ctor_sig.clone());
    module.add_str("ui_status_bar", ui_status_bar, ctor_sig.clone());
    module.add_str("ui_toolbar", ui_toolbar, ctor_sig.clone());
    module.add_str("ui_progress_bar", ui_progress_bar, ctor_sig.clone());
    module.add_str("ui_slider", ui_slider, ctor_sig.clone());
    module.add_str("ui_grid_view", ui_grid_view, ctor_sig.clone());
    module.add_str("ui_color_picker", ui_color_picker, ctor_sig.clone());
    module.add_str("ui_flow_text", ui_flow_text, ctor_sig);

    let text_sig = Dfn::nl(vec![widget.clone(), Type::Str], Type::Void);
    module.add_str("ui_set_text", ui_set_text, text_sig.clone());
    module.add_str("ui_set_tooltip", ui_set_tooltip, text_sig.clone());
    module.add_str("ui_set_placeholder", ui_set_placeholder, text_sig.clone());
    module.add_str("ui_set_color", ui_set_color, text_sig.clone());
    module.add_str("ui_set_orientation", ui_set_orientation, text_sig.clone());
    module.add_str("ui_set_scroll_bars", ui_set_scroll_bars, text_sig.clone());
    module.add_str("ui_set_selected", ui_set_selected, text_sig);

    let flag_sig = Dfn::nl(vec![widget.clone(), Type::Bool], Type::Void);
    module.add_str("ui_set_enabled", ui_set_enabled, flag_sig.clone());
    module.add_str("ui_set_visible", ui_set_visible, flag_sig.clone());
    module.add_str("ui_set_multiline", ui_set_multiline, flag_sig.clone());
    module.add_str("ui_set_password", ui_set_password, flag_sig.clone());
    module.add_str("ui_set_editable", ui_set_editable, flag_sig.clone());
    module.add_str("ui_set_border", ui_set_border, flag_sig.clone());
    module.add_str("ui_set_wrap", ui_set_wrap, flag_sig.clone());
    module.add_str("ui_set_checked", ui_set_checked, flag_sig);

    let list_sig = Dfn::nl(vec![widget.clone(), text_array.clone()], Type::Void);
    module.add_str("ui_set_items", ui_set_items, list_sig.clone());
    module.add_str("ui_set_columns", ui_set_columns, list_sig);

    module.add_str(
        "ui_set_range",
        ui_set_range,
        Dfn::nl(vec![widget.clone(), Type::F64, Type::F64], Type::Void),
    );
    module.add_str(
        "ui_set_value",
        ui_set_value,
        Dfn::nl(vec![widget.clone(), Type::F64], Type::Void),
    );
    module.add_str(
        "ui_set_selected_index",
        ui_set_selected_index,
        Dfn::nl(vec![widget.clone(), Type::F64], Type::Void),
    );
    module.add_str(
        "ui_anchor",
        ui_anchor,
        Dfn::nl(vec![widget.clone(), Type::Str], Type::Void),
    );
    module.add_str(
        "ui_on",
        ui_on,
        Dfn::nl(vec![widget.clone(), Type::Str, Type::Str], Type::Void),
    );
    // Test hook: a program with no input injection queues one dispatch. The
    // host runs it once the loop starts, never re-entering Dyon.
    module.add_str(
        "ui_invoke",
        ui_invoke,
        Dfn::nl(vec![widget, Type::Str], Type::Void),
    );
}

/// An ad-hoc type named `name`, so a plan returned by one binding is accepted
/// only by bindings that expect the same kind of plan.
fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}

/// The constructor/setter vocabulary is expanded to non-capturing `fn`
/// pointers because [`Module::add_str`] takes a function pointer, not a closure.
macro_rules! widget_binding {
    ($name:ident => $kind:expr) => {
        fn $name(rt: &mut Runtime) -> Result<Variable, String> {
            constructor(rt, $kind)
        }
    };
}

widget_binding!(ui_button => WidgetKind::Button);
widget_binding!(ui_edit => WidgetKind::Edit);
widget_binding!(ui_label => WidgetKind::Label);
widget_binding!(ui_check_box => WidgetKind::CheckBox);
widget_binding!(ui_radio_group => WidgetKind::RadioGroup);
widget_binding!(ui_combo_box => WidgetKind::ComboBox);
widget_binding!(ui_list_view => WidgetKind::ListView);
widget_binding!(ui_tree_view => WidgetKind::TreeView);
widget_binding!(ui_group_box => WidgetKind::GroupBox);
widget_binding!(ui_panel => WidgetKind::Panel);
widget_binding!(ui_scroll_view => WidgetKind::ScrollView);
widget_binding!(ui_tabs => WidgetKind::Tabs);
widget_binding!(ui_menu => WidgetKind::Menu);
widget_binding!(ui_status_bar => WidgetKind::StatusBar);
widget_binding!(ui_toolbar => WidgetKind::Toolbar);
widget_binding!(ui_progress_bar => WidgetKind::ProgressBar);
widget_binding!(ui_slider => WidgetKind::Slider);
widget_binding!(ui_grid_view => WidgetKind::GridView);
widget_binding!(ui_color_picker => WidgetKind::ColorPicker);
widget_binding!(ui_flow_text => WidgetKind::FlowText);

macro_rules! text_binding {
    ($name:ident => $label:literal) => {
        fn $name(rt: &mut Runtime) -> Result<(), String> {
            text_setter(rt, $label)
        }
    };
}

text_binding!(ui_set_text => "ui_set_text");
text_binding!(ui_set_tooltip => "ui_set_tooltip");
text_binding!(ui_set_placeholder => "ui_set_placeholder");
text_binding!(ui_set_color => "ui_set_color");
text_binding!(ui_set_orientation => "ui_set_orientation");
text_binding!(ui_set_scroll_bars => "ui_set_scroll_bars");
text_binding!(ui_set_selected => "ui_set_selected");

macro_rules! flag_binding {
    ($name:ident => $label:literal) => {
        fn $name(rt: &mut Runtime) -> Result<(), String> {
            flag_setter(rt, $label)
        }
    };
}

flag_binding!(ui_set_enabled => "ui_set_enabled");
flag_binding!(ui_set_visible => "ui_set_visible");
flag_binding!(ui_set_multiline => "ui_set_multiline");
flag_binding!(ui_set_password => "ui_set_password");
flag_binding!(ui_set_editable => "ui_set_editable");
flag_binding!(ui_set_border => "ui_set_border");
flag_binding!(ui_set_wrap => "ui_set_wrap");
flag_binding!(ui_set_checked => "ui_set_checked");

macro_rules! list_binding {
    ($name:ident => $label:literal) => {
        fn $name(rt: &mut Runtime) -> Result<(), String> {
            list_setter(rt, $label)
        }
    };
}

list_binding!(ui_set_items => "ui_set_items");
list_binding!(ui_set_columns => "ui_set_columns");

fn widget_variable(id: plan::HandleId) -> Variable {
    plan::handle_variable(id)
}

fn ui_window(rt: &mut Runtime) -> Result<Variable, String> {
    let height: f64 = rt.pop()?;
    let width: f64 = rt.pop()?;
    let title: String = rt.pop()?;
    let id = plan::window(title, width as f32, height as f32)?;
    Ok(widget_variable(id))
}

fn ui_free(rt: &mut Runtime) -> Result<Variable, String> {
    let height: f64 = rt.pop()?;
    let width: f64 = rt.pop()?;
    let id = plan::free(width as f32, height as f32)?;
    Ok(widget_variable(id))
}

fn ui_add(rt: &mut Runtime) -> Result<(), String> {
    let child = plan::pop_handle(rt)?;
    let root = plan::pop_handle(rt)?;
    plan::add(root, child)
}

fn constructor(rt: &mut Runtime, kind: WidgetKind) -> Result<Variable, String> {
    let height: f64 = rt.pop()?;
    let width: f64 = rt.pop()?;
    let y: f64 = rt.pop()?;
    let x: f64 = rt.pop()?;
    let text: String = rt.pop()?;
    let id = plan::widget(kind, text, x as f32, y as f32, width as f32, height as f32)?;
    Ok(widget_variable(id))
}

fn ui_run(rt: &mut Runtime) -> Result<(), String> {
    let root = plan::pop_handle(rt)?;
    let window = plan::pop_handle(rt)?;
    plan::mark_ran(window, root)
}

fn text_setter(rt: &mut Runtime, name: &'static str) -> Result<(), String> {
    let value: String = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        match name {
            "ui_set_text" => widget.text = value.clone(),
            "ui_set_tooltip" => widget.tooltip = Some(value.clone()),
            "ui_set_placeholder" => widget.setters.placeholder = Some(value.clone()),
            "ui_set_color" => widget.setters.color = Some(value.clone()),
            "ui_set_orientation" => widget.setters.orientation = Some(value.clone()),
            "ui_set_scroll_bars" => widget.setters.scroll_bars = Some(value.clone()),
            "ui_set_selected" => widget.setters.selected = Some(value.clone()),
            _ => return Err(format!("unhandled text setter `{name}`")),
        }
        Ok(())
    })?;
    // A handler may set text while the window is live; apply it there too.
    if name == "ui_set_text" {
        super::host::set_live_text(id, &value);
    }
    Ok(())
}

fn flag_setter(rt: &mut Runtime, name: &'static str) -> Result<(), String> {
    let value: bool = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        match name {
            "ui_set_enabled" => widget.enabled = value,
            "ui_set_visible" => widget.visible = value,
            "ui_set_multiline" => widget.setters.multiline = Some(value),
            "ui_set_password" => widget.setters.password = Some(value),
            "ui_set_editable" => widget.setters.editable = Some(value),
            "ui_set_border" => widget.setters.border = Some(value),
            "ui_set_wrap" => widget.setters.wrap = Some(value),
            "ui_set_checked" => widget.setters.checked = Some(value),
            _ => return Err(format!("unhandled flag setter `{name}`")),
        }
        Ok(())
    })?;
    // A handler may call a setter while the window is live; apply it there too.
    match name {
        "ui_set_enabled" => super::host::set_live_enabled(id, value),
        "ui_set_visible" => super::host::set_live_visible(id, value),
        _ => {}
    }
    Ok(())
}

fn list_setter(rt: &mut Runtime, name: &'static str) -> Result<(), String> {
    let items = pop_string_array(rt)?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        if name == "ui_set_columns" {
            widget.setters.columns = Some(items);
        } else {
            widget.setters.items = Some(items);
        }
        Ok(())
    })
}

fn ui_set_range(rt: &mut Runtime) -> Result<(), String> {
    let max: f64 = rt.pop()?;
    let min: f64 = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        widget.setters.range = Some((min, max));
        Ok(())
    })
}

fn ui_set_value(rt: &mut Runtime) -> Result<(), String> {
    let value: f64 = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        widget.setters.value = Some(value);
        Ok(())
    })
}

fn ui_set_selected_index(rt: &mut Runtime) -> Result<(), String> {
    let index: f64 = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::setters(id, |widget| {
        widget.setters.selected_index = Some(index.max(0.0) as usize);
        Ok(())
    })
}

fn ui_anchor(rt: &mut Runtime) -> Result<(), String> {
    let name: String = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::set_anchor(id, &name)
}

fn ui_on(rt: &mut Runtime) -> Result<(), String> {
    let handler: String = rt.pop()?;
    let event: String = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::on(id, &event, &handler)
}

fn ui_invoke(rt: &mut Runtime) -> Result<(), String> {
    let event: String = rt.pop()?;
    let id = plan::pop_handle(rt)?;
    plan::invoke(id, &event)
}

/// Reads `[text, ...]` into owned strings, rejecting a non-string element.
fn pop_string_array(rt: &mut Runtime) -> Result<Vec<String>, String> {
    let value: Variable = rt.pop()?;
    let Variable::Array(items) = value else {
        return Err(format!(
            "expected an array of strings, got {}",
            value.typeof_var()
        ));
    };
    items
        .iter()
        .map(|item| match item {
            Variable::Str(text) => Ok(text.to_string()),
            other => Err(format!(
                "expected a string item, got {}",
                other.typeof_var()
            )),
        })
        .collect()
}
