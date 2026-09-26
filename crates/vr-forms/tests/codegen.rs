//! Codegen tests: golden output, idempotence, escaping and a compile-only
//! check against stub signatures for the target `ui_*` surface.

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};
use vr_forms::codegen::{CodegenOptions, EmitHandlers, Indent, generate};
use vr_forms::model::{
    Anchor, Bounds, CheckBoxProps, ChoiceProps, ColorPickerProps, Column, ComboBoxProps, Control,
    ControlKind, Dip, EditProps, FlowTextProps, Form, ListProps, MenuItem, MenuProps, Orientation,
    PanelProps, RangeProps, ScrollBars, ScrollViewProps, Size, SliderProps, StatusBarProps,
    TabsProps, ToolbarButton, ToolbarProps, TreeNode, TreeViewProps,
};

const ANCHORS: [Anchor; 12] = [
    Anchor::TopLeft,
    Anchor::Top,
    Anchor::TopRight,
    Anchor::Left,
    Anchor::Center,
    Anchor::Right,
    Anchor::BottomLeft,
    Anchor::Bottom,
    Anchor::BottomRight,
    Anchor::StretchHorizontal,
    Anchor::StretchVertical,
    Anchor::Fill,
];

const NAMES: [&str; 20] = [
    "Greet",
    "NameEdit",
    "NameLabel",
    "EnabledBox",
    "SizeRadio",
    "Choice",
    "Items",
    "Tree",
    "Options",
    "Surface",
    "Scroller",
    "Pages",
    "MenuBar",
    "Status",
    "Tools",
    "Progress",
    "Level",
    "Grid",
    "Colour",
    "Flow",
];

/// One form exercising every `ControlKind`, every anchor and the common
/// non-default properties, laid out in a grid that fits the 640x480 surface.
fn sample_form() -> Form {
    let kinds = vec![
        (ControlKind::Button, "Greet"),
        (
            ControlKind::Edit(EditProps {
                multiline: true,
                password: false,
                placeholder: Some("name".to_owned()),
            }),
            "hello",
        ),
        (ControlKind::Label, "Name:"),
        (
            ControlKind::CheckBox(CheckBoxProps { checked: true }),
            "Enabled",
        ),
        (
            ControlKind::RadioGroup(ChoiceProps {
                items: vec!["Small".to_owned(), "Large".to_owned()],
                selected: Some("Large".to_owned()),
            }),
            "Size",
        ),
        (
            ControlKind::ComboBox(ComboBoxProps {
                items: vec!["A".to_owned(), "B".to_owned()],
                selected: Some("A".to_owned()),
                editable: true,
            }),
            "Choice",
        ),
        (
            ControlKind::ListView(ListProps {
                columns: vec![Column {
                    title: "Name".to_owned(),
                    width: Dip::new(120.0),
                }],
                items: vec!["row".to_owned()],
            }),
            "",
        ),
        (
            ControlKind::TreeView(TreeViewProps {
                nodes: vec![TreeNode {
                    label: "root".to_owned(),
                    children: vec![TreeNode {
                        label: "leaf".to_owned(),
                        children: Vec::new(),
                    }],
                }],
            }),
            "",
        ),
        (ControlKind::GroupBox, "Options"),
        (ControlKind::Panel(PanelProps { border: true }), ""),
        (
            ControlKind::ScrollView(ScrollViewProps {
                bars: ScrollBars::Both,
            }),
            "",
        ),
        (
            ControlKind::Tabs(TabsProps {
                tabs: vec!["One".to_owned(), "Two".to_owned()],
                selected: 1,
            }),
            "",
        ),
        (
            ControlKind::Menu(MenuProps {
                items: vec![
                    MenuItem {
                        label: "File".to_owned(),
                        shortcut: Some("Ctrl+F".to_owned()),
                        separator: false,
                    },
                    MenuItem {
                        label: String::new(),
                        shortcut: None,
                        separator: true,
                    },
                    MenuItem {
                        label: "Edit".to_owned(),
                        shortcut: None,
                        separator: false,
                    },
                ],
            }),
            "",
        ),
        (
            ControlKind::StatusBar(StatusBarProps {
                parts: vec!["Ready".to_owned()],
            }),
            "",
        ),
        (
            ControlKind::Toolbar(ToolbarProps {
                buttons: vec![ToolbarButton {
                    text: "Run".to_owned(),
                    tooltip: Some("Run the app".to_owned()),
                    separator: false,
                }],
            }),
            "",
        ),
        (
            ControlKind::ProgressBar(RangeProps {
                min: 0.0,
                max: 100.0,
                value: 42.0,
            }),
            "",
        ),
        (
            ControlKind::Slider(SliderProps {
                min: 0.0,
                max: 10.0,
                value: 5.0,
                orientation: Orientation::Vertical,
            }),
            "",
        ),
        (
            ControlKind::GridView(ListProps {
                columns: vec![Column {
                    title: "Col".to_owned(),
                    width: Dip::new(90.0),
                }],
                items: Vec::new(),
            }),
            "",
        ),
        (
            ControlKind::ColorPicker(ColorPickerProps {
                color: Some("#a1b2c3".to_owned()),
            }),
            "",
        ),
        (
            ControlKind::FlowText(FlowTextProps { wrap: false }),
            "Flow \"text\"\nline",
        ),
    ];

    let controls = kinds
        .into_iter()
        .zip(NAMES)
        .enumerate()
        .map(|(index, ((kind, text), name))| {
            let column = (index % 6) as f64;
            let row = (index / 6) as f64;
            Control {
                kind,
                name: name.to_owned(),
                bounds: Bounds {
                    x: Dip::new(10.0 + column * 100.0),
                    y: Dip::new(10.0 + row * 60.0),
                    width: Dip::new(90.0),
                    height: Dip::new(40.0),
                },
                text: text.to_owned(),
                enabled: index % 5 != 0,
                visible: index % 7 != 3,
                tooltip: (index % 4 == 1).then(|| format!("tip {name}")),
                anchor: ANCHORS[index % ANCHORS.len()],
            }
        })
        .collect();

    Form {
        name: "Main".to_owned(),
        size: Size {
            width: Dip::new(640.0),
            height: Dip::new(480.0),
        },
        controls,
    }
}

#[test]
fn golden_matches_all_kinds_output() {
    let generated = generate(&sample_form(), CodegenOptions::default());
    // The generator always emits LF; a Windows checkout may hand `include_str!`
    // the golden file as CRLF, so normalise before comparing.
    let golden = include_str!("golden/all_kinds.dyon").replace("\r\n", "\n");
    assert_eq!(generated, golden);
}

#[test]
fn regeneration_is_byte_identical() {
    let form = sample_form();
    let options = CodegenOptions::default();
    let first = generate(&form, options.clone());
    let second = generate(&form, options);
    assert_eq!(first, second, "generate is deterministic");

    let reparsed = Form::from_json(&form.to_json().expect("serialize")).expect("parse");
    assert_eq!(
        generate(&reparsed, CodegenOptions::default()),
        first,
        "a JSON round trip does not change the output"
    );
}

#[test]
fn escapes_text_in_the_generated_source() {
    let control = Control {
        kind: ControlKind::Edit(EditProps::default()),
        name: "Special".to_owned(),
        bounds: Bounds {
            x: Dip::new(0.0),
            y: Dip::new(0.0),
            width: Dip::new(90.0),
            height: Dip::new(40.0),
        },
        text: "a\"b\\c\nd café ☃".to_owned(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor: Anchor::TopLeft,
    };
    let form = Form {
        name: "Esc".to_owned(),
        size: Size {
            width: Dip::new(200.0),
            height: Dip::new(100.0),
        },
        controls: vec![control],
    };

    let generated = generate(&form, CodegenOptions::default());
    assert!(
        generated.contains(r#"ui_set_text(c0, "a\"b\\c\nd café ☃")"#),
        "{generated}"
    );
}

#[test]
fn handler_stubs_can_be_suppressed() {
    let options = CodegenOptions {
        handlers: EmitHandlers::None,
        ..CodegenOptions::default()
    };
    let generated = generate(&sample_form(), options);
    assert!(!generated.contains("fn on_"), "{generated}");
    assert_eq!(generated.matches("fn ").count(), 1, "only main remains");
}

#[test]
fn indentation_option_changes_the_body() {
    let options = CodegenOptions {
        indent: Indent::Spaces(2),
        ..CodegenOptions::default()
    };
    let generated = generate(&sample_form(), options);
    assert!(generated.contains("\n  w := ui_window("));
    assert!(!generated.contains("\n    w := ui_window("));
}

#[test]
fn module_name_labels_the_header() {
    let options = CodegenOptions {
        module: "Startup".to_owned(),
        ..CodegenOptions::default()
    };
    assert!(
        generate(&sample_form(), options).starts_with("// Generated by vr-forms from `Startup`.")
    );
}

#[test]
fn generated_source_compiles() {
    let source = generate(&sample_form(), CodegenOptions::default());
    let mut module = Module::new();
    register_target_api(&mut module);
    dyon::load_str("generated.dyon", Arc::new(source), &mut module)
        .expect("generated Dyon must parse and pass the lifetime checker");
}

/// Stub signatures for the target `ui_*` surface, so the generated program can
/// be parsed and pass Dyon's lifetime/type checker without the real bindings.
fn register_target_api(module: &mut Module) {
    fn stub(_rt: &mut Runtime) -> Result<Variable, String> {
        Err("stub".to_owned())
    }

    let window = ad_hoc("Window");
    let widget = ad_hoc("Widget");
    let free = ad_hoc("Free");
    let text_array = Type::Array(Box::new(Type::Str));

    module.add_str(
        "ui_window",
        stub,
        Dfn::nl(vec![Type::Str, Type::F64, Type::F64], window.clone()),
    );
    module.add_str(
        "ui_free",
        stub,
        Dfn::nl(vec![Type::F64, Type::F64], free.clone()),
    );
    module.add_str(
        "ui_add",
        stub,
        Dfn::nl(vec![free.clone(), widget.clone()], Type::Void),
    );
    module.add_str("ui_run", stub, Dfn::nl(vec![window, free], Type::Void));

    let constructors = [
        "ui_button",
        "ui_edit",
        "ui_label",
        "ui_check_box",
        "ui_radio_group",
        "ui_combo_box",
        "ui_list_view",
        "ui_tree_view",
        "ui_group_box",
        "ui_panel",
        "ui_scroll_view",
        "ui_tabs",
        "ui_menu",
        "ui_status_bar",
        "ui_toolbar",
        "ui_progress_bar",
        "ui_slider",
        "ui_grid_view",
        "ui_color_picker",
        "ui_flow_text",
    ];
    for name in constructors {
        module.add_str(
            name,
            stub,
            Dfn::nl(
                vec![Type::Str, Type::F64, Type::F64, Type::F64, Type::F64],
                widget.clone(),
            ),
        );
    }

    let text_setters = [
        "ui_set_text",
        "ui_set_tooltip",
        "ui_set_placeholder",
        "ui_set_color",
        "ui_set_orientation",
        "ui_set_scroll_bars",
        "ui_set_selected",
    ];
    for name in text_setters {
        module.add_str(
            name,
            stub,
            Dfn::nl(vec![widget.clone(), Type::Str], Type::Void),
        );
    }

    let flag_setters = [
        "ui_set_enabled",
        "ui_set_visible",
        "ui_set_multiline",
        "ui_set_password",
        "ui_set_editable",
        "ui_set_border",
        "ui_set_wrap",
        "ui_set_checked",
    ];
    for name in flag_setters {
        module.add_str(
            name,
            stub,
            Dfn::nl(vec![widget.clone(), Type::Bool], Type::Void),
        );
    }

    for name in ["ui_set_items", "ui_set_columns"] {
        module.add_str(
            name,
            stub,
            Dfn::nl(vec![widget.clone(), text_array.clone()], Type::Void),
        );
    }

    module.add_str(
        "ui_set_range",
        stub,
        Dfn::nl(vec![widget.clone(), Type::F64, Type::F64], Type::Void),
    );
    module.add_str(
        "ui_set_value",
        stub,
        Dfn::nl(vec![widget.clone(), Type::F64], Type::Void),
    );
    module.add_str(
        "ui_set_selected_index",
        stub,
        Dfn::nl(vec![widget.clone(), Type::F64], Type::Void),
    );
    module.add_str(
        "ui_anchor",
        stub,
        Dfn::nl(vec![widget.clone(), Type::Str], Type::Void),
    );
    module.add_str(
        "ui_on",
        stub,
        Dfn::nl(vec![widget, Type::Str, Type::Str], Type::Void),
    );
}

fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}
