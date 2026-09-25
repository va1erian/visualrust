use super::*;
use crate::model::{
    CheckBoxProps, ChoiceProps, ColorPickerProps, Column, ComboBoxProps, EditProps, FlowTextProps,
    ListProps, MenuItem, MenuProps, PanelProps, RangeProps, ScrollViewProps, SliderProps,
    StatusBarProps, TabsProps, ToolbarButton, ToolbarProps, TreeNode, TreeViewProps,
};

fn dip(value: f64) -> Dip {
    Dip::new(value)
}

fn bounds(x: f64, y: f64) -> Bounds {
    Bounds {
        x: dip(x),
        y: dip(y),
        width: dip(80.0),
        height: dip(40.0),
    }
}

fn control(kind: ControlKind, name: &str, x: f64, y: f64) -> Control {
    Control {
        kind,
        name: name.to_owned(),
        bounds: bounds(x, y),
        text: String::new(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor: Anchor::TopLeft,
    }
}

fn button(name: &str) -> Control {
    control(ControlKind::Button, name, 10.0, 10.0)
}

fn form_with(controls: Vec<Control>) -> Form {
    Form {
        name: "Main".to_owned(),
        size: Size {
            width: dip(640.0),
            height: dip(480.0),
        },
        controls,
    }
}

/// Every palette widget, laid out in a grid that fits the 640x480 test form so
/// the round-trip and validation tests share one exhaustive fixture.
fn all_kinds_form() -> Form {
    let kinds = vec![
        ControlKind::Button,
        ControlKind::Edit(EditProps {
            multiline: true,
            password: false,
            placeholder: Some("name".to_owned()),
        }),
        ControlKind::Label,
        ControlKind::CheckBox(CheckBoxProps { checked: true }),
        ControlKind::RadioGroup(ChoiceProps {
            items: vec!["Small".to_owned(), "Large".to_owned()],
            selected: Some("Large".to_owned()),
        }),
        ControlKind::ComboBox(ComboBoxProps {
            items: vec!["A".to_owned(), "B".to_owned()],
            selected: Some("A".to_owned()),
            editable: true,
        }),
        ControlKind::ListView(ListProps {
            columns: vec![Column {
                title: "Name".to_owned(),
                width: dip(120.0),
            }],
            items: vec!["row".to_owned()],
        }),
        ControlKind::TreeView(TreeViewProps {
            nodes: vec![TreeNode {
                label: "root".to_owned(),
                children: vec![TreeNode {
                    label: "leaf".to_owned(),
                    children: Vec::new(),
                }],
            }],
        }),
        ControlKind::GroupBox,
        ControlKind::Panel(PanelProps { border: true }),
        ControlKind::ScrollView(ScrollViewProps {
            bars: ScrollBars::Both,
        }),
        ControlKind::Tabs(TabsProps {
            tabs: vec!["One".to_owned(), "Two".to_owned()],
            selected: 1,
        }),
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
            ],
        }),
        ControlKind::StatusBar(StatusBarProps {
            parts: vec!["Ready".to_owned()],
        }),
        ControlKind::Toolbar(ToolbarProps {
            buttons: vec![ToolbarButton {
                text: "Run".to_owned(),
                tooltip: Some("Run the app".to_owned()),
                separator: false,
            }],
        }),
        ControlKind::ProgressBar(RangeProps {
            min: 0.0,
            max: 100.0,
            value: 42.0,
        }),
        ControlKind::Slider(SliderProps {
            min: 0.0,
            max: 10.0,
            value: 5.0,
            orientation: Orientation::Vertical,
        }),
        ControlKind::GridView(ListProps {
            columns: vec![Column {
                title: "Col".to_owned(),
                width: dip(90.0),
            }],
            items: Vec::new(),
        }),
        ControlKind::ColorPicker(ColorPickerProps {
            color: Some("#a1b2c3".to_owned()),
        }),
        ControlKind::FlowText(FlowTextProps { wrap: false }),
    ];

    let controls = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            let x = 10.0 + (index % 6) as f64 * 100.0;
            let y = 10.0 + (index / 6) as f64 * 60.0;
            control(kind, &format!("c{index}"), x, y)
        })
        .collect();
    form_with(controls)
}

#[test]
fn all_kinds_round_trip_through_json() {
    let form = all_kinds_form();
    form.validate().expect("fixture is valid");

    let text = form.to_json().expect("serialize");
    let parsed = Form::from_json(&text).expect("parse");
    assert_eq!(form, parsed);
    assert_eq!(parsed.controls.len(), 20);
}

#[test]
fn serialization_is_byte_stable() {
    let form = all_kinds_form();
    let first = form.to_json().expect("serialize");
    let second = Form::from_json(&first)
        .expect("parse")
        .to_json()
        .expect("re-serialize");
    assert_eq!(first, second);
}

#[test]
fn json_keeps_declaration_order_and_kind_tags() {
    let text = all_kinds_form().to_json().expect("serialize");
    assert!(text.contains("\"type\": \"color_picker\""));
    assert!(text.contains("\"type\": \"flow_text\""));

    let button = text.find("c0").expect("first control");
    let last = text.find("c19").expect("last control");
    assert!(button < last, "controls keep z-order");
}

#[test]
fn save_and_load_round_trips_through_a_file() {
    let dir = std::env::temp_dir().join(format!("vr-forms-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("main.vrform");

    let form = all_kinds_form();
    form.save(&path).expect("save");
    let loaded = Form::load(&path).expect("load");

    assert_eq!(form, loaded);
    std::fs::remove_dir_all(&dir).expect("clean temp dir");
}

#[test]
fn unknown_form_fields_are_rejected() {
    let text = r#"{ "name": "Main", "size": { "width": 1.0, "height": 1.0 }, "bogus": 1 }"#;
    assert!(matches!(Form::from_json(text), Err(FormError::Parse(_))));
}

#[test]
fn unknown_control_fields_are_rejected() {
    let text = r#"{
        "name": "Main",
        "size": { "width": 100.0, "height": 100.0 },
        "controls": [{
            "kind": { "type": "button" },
            "name": "ok",
            "bounds": { "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 },
            "bogus": true
        }]
    }"#;
    assert!(matches!(Form::from_json(text), Err(FormError::Parse(_))));
}

#[test]
fn unknown_control_kind_is_rejected() {
    let text = r#"{
        "name": "Main",
        "size": { "width": 100.0, "height": 100.0 },
        "controls": [{
            "kind": { "type": "hologram" },
            "name": "ok",
            "bounds": { "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }
        }]
    }"#;
    assert!(matches!(Form::from_json(text), Err(FormError::Parse(_))));
}

#[test]
fn invalid_enum_is_rejected_at_parse_time() {
    let text = r#"{
        "name": "Main",
        "size": { "width": 100.0, "height": 100.0 },
        "controls": [{
            "kind": { "type": "button" },
            "name": "ok",
            "bounds": { "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 },
            "anchor": "sideways"
        }]
    }"#;
    assert!(matches!(Form::from_json(text), Err(FormError::Parse(_))));
}

#[test]
fn empty_form_name_is_reported() {
    let mut form = form_with(Vec::new());
    form.name = "  ".to_owned();
    assert_eq!(form.validate(), Err(ValidationError::EmptyFormName));
}

#[test]
fn invalid_form_name_is_reported() {
    let mut form = form_with(Vec::new());
    form.name = "bad name".to_owned();
    assert_eq!(
        form.validate(),
        Err(ValidationError::InvalidFormName {
            name: "bad name".to_owned(),
        })
    );
}

#[test]
fn non_positive_form_size_is_reported() {
    let mut form = form_with(Vec::new());
    form.size.width = dip(0.0);
    assert_eq!(
        form.validate(),
        Err(ValidationError::InvalidFormSize {
            width: 0.0,
            height: 480.0,
        })
    );
}

#[test]
fn empty_control_name_is_reported() {
    let form = form_with(vec![button("")]);
    assert_eq!(
        form.validate(),
        Err(ValidationError::EmptyControlName { index: 0 })
    );
}

#[test]
fn invalid_control_name_is_reported() {
    let form = form_with(vec![button("bad name")]);
    assert_eq!(
        form.validate(),
        Err(ValidationError::InvalidControlName {
            name: "bad name".to_owned(),
        })
    );
}

#[test]
fn duplicate_control_names_are_reported() {
    let form = form_with(vec![button("ok"), button("ok")]);
    assert_eq!(
        form.validate(),
        Err(ValidationError::DuplicateControlName {
            name: "ok".to_owned(),
        })
    );
}

#[test]
fn non_positive_control_size_is_reported() {
    let mut ctrl = button("ok");
    ctrl.bounds.width = dip(0.0);
    assert_eq!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::InvalidControlSize {
            name: "ok".to_owned(),
            kind: "button",
        })
    );
}

#[test]
fn control_outside_the_form_is_reported() {
    let mut ctrl = button("ok");
    ctrl.bounds = bounds(620.0, 10.0);
    assert!(matches!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::ControlOutOfBounds { .. })
    ));
}

#[test]
fn negative_position_is_out_of_bounds() {
    let mut ctrl = button("ok");
    ctrl.bounds = bounds(-1.0, 10.0);
    assert!(matches!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::ControlOutOfBounds { .. })
    ));
}

#[test]
fn invalid_range_is_reported() {
    let ctrl = control(
        ControlKind::ProgressBar(RangeProps {
            min: 0.0,
            max: 10.0,
            value: 11.0,
        }),
        "bar",
        10.0,
        10.0,
    );
    assert!(matches!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::InvalidRange { .. })
    ));
}

#[test]
fn selection_index_out_of_range_is_reported() {
    let ctrl = control(
        ControlKind::Tabs(TabsProps {
            tabs: vec!["one".to_owned()],
            selected: 3,
        }),
        "tabs",
        10.0,
        10.0,
    );
    assert_eq!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::SelectionOutOfRange {
            name: "tabs".to_owned(),
            kind: "tabs",
            selected: 3,
            len: 1,
        })
    );
}

#[test]
fn unknown_selection_is_reported() {
    let ctrl = control(
        ControlKind::RadioGroup(ChoiceProps {
            items: vec!["a".to_owned()],
            selected: Some("z".to_owned()),
        }),
        "radio",
        10.0,
        10.0,
    );
    assert_eq!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::UnknownSelection {
            name: "radio".to_owned(),
            kind: "radio_group",
            value: "z".to_owned(),
        })
    );
}

#[test]
fn empty_choice_items_are_reported() {
    let ctrl = control(
        ControlKind::RadioGroup(ChoiceProps {
            items: Vec::new(),
            selected: None,
        }),
        "radio",
        10.0,
        10.0,
    );
    assert_eq!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::NoItems {
            name: "radio".to_owned(),
            kind: "radio_group",
        })
    );
}

#[test]
fn invalid_color_is_reported() {
    let ctrl = control(
        ControlKind::ColorPicker(ColorPickerProps {
            color: Some("#zzzzzz".to_owned()),
        }),
        "color",
        10.0,
        10.0,
    );
    assert_eq!(
        form_with(vec![ctrl]).validate(),
        Err(ValidationError::InvalidColor {
            name: "color".to_owned(),
            value: "#zzzzzz".to_owned(),
        })
    );
}

#[test]
fn too_many_controls_do_not_collide_in_names() {
    let controls = (0..30).map(|index| button(&format!("c{index}"))).collect();
    form_with(controls).validate().expect("unique names");
}
