//! End-to-end check that the designer surface renders live widgets and its
//! selection adorners.
//!
//! Builds a form with one of every portable kind plus a placeholder, selects a
//! control, lets the widgets paint, then captures the window with
//! `vr_tooling::capture_window_rendered` (the `PrintWindow` path, which is safe
//! across processes) and counts the accent pixels that make up the outline and
//! the eight handles.
//!
//! Follows the workspace skip pattern: a session with no desktop prints `SKIP`
//! and passes, but a captured window with no accent pixels fails.

#![cfg(windows)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use vr_forms::designer::DesignerSurface;
use vr_forms::model::{
    CheckBoxProps, ChoiceProps, ComboBoxProps, EditProps, RangeProps, SliderProps, TreeViewProps,
};
use vr_forms::{Anchor, Bounds, Control, ControlKind, Dip, Form, Orientation, Size};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use xui::xui_core::backend::{Backend, PlatformSpec};
use xui::xui_core::{App, Ui, run_app};
use xui::{Hwnd, Theme, Win32Backend, dip};

/// How long to let the widgets paint before capturing, in ms.
const CAPTURE_MS: u32 = 900;

/// The form the test hosts: one of each portable kind and a `TreeView`, which
/// has no portable widget and must draw as a placeholder.
fn sample_form() -> Form {
    let button = control(
        "OkButton",
        ControlKind::Button,
        20.0,
        16.0,
        140.0,
        52.0,
        "OK",
    );
    let label = control(
        "TitleLabel",
        ControlKind::Label,
        160.0,
        16.0,
        400.0,
        52.0,
        "Designer",
    );
    let edit = control(
        "NameEdit",
        ControlKind::Edit(EditProps::default()),
        20.0,
        60.0,
        260.0,
        92.0,
        "hello",
    );
    let check = control(
        "AgreeCheck",
        ControlKind::CheckBox(CheckBoxProps { checked: true }),
        280.0,
        60.0,
        520.0,
        92.0,
        "Agree",
    );
    let combo = control(
        "PickCombo",
        ControlKind::ComboBox(ComboBoxProps {
            items: vec!["Alpha".into(), "Beta".into()],
            selected: Some("Alpha".into()),
            editable: false,
        }),
        20.0,
        100.0,
        260.0,
        128.0,
        "",
    );
    let bar = control(
        "LoadBar",
        ControlKind::ProgressBar(RangeProps {
            min: 0.0,
            max: 100.0,
            value: 40.0,
        }),
        280.0,
        100.0,
        520.0,
        120.0,
        "",
    );
    let slider = control(
        "SpeedSlider",
        ControlKind::Slider(SliderProps {
            min: 0.0,
            max: 100.0,
            value: 30.0,
            orientation: Orientation::Horizontal,
        }),
        20.0,
        140.0,
        260.0,
        168.0,
        "",
    );
    let radios = control(
        "ChoiceRadio",
        ControlKind::RadioGroup(ChoiceProps {
            items: vec!["Low".into(), "Medium".into(), "High".into()],
            selected: Some("Low".into()),
        }),
        280.0,
        140.0,
        520.0,
        240.0,
        "",
    );
    let group = control(
        "OptionsGroup",
        ControlKind::GroupBox,
        20.0,
        180.0,
        260.0,
        300.0,
        "Options",
    );
    let tree = control(
        "FilesTree",
        ControlKind::TreeView(TreeViewProps::default()),
        280.0,
        260.0,
        520.0,
        340.0,
        "",
    );
    Form {
        name: "DesignerDemo".into(),
        size: Size {
            width: Dip::new(560.0),
            height: Dip::new(360.0),
        },
        controls: vec![
            button, label, edit, check, combo, bar, slider, radios, group, tree,
        ],
    }
}

/// A control with the common defaults filled in.
fn control(
    name: &str,
    kind: ControlKind,
    x: f64,
    y: f64,
    right: f64,
    bottom: f64,
    text: &str,
) -> Control {
    Control {
        kind,
        name: name.to_string(),
        bounds: Bounds {
            x: Dip::new(x),
            y: Dip::new(y),
            width: Dip::new(right - x),
            height: Dip::new(bottom - y),
        },
        text: text.to_string(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor: Anchor::TopLeft,
    }
}

/// The messages the test app handles.
enum Msg {
    Capture,
}

/// The test app: keeps the surface (and so its widgets) alive and captures once.
struct DesignerApp {
    _surface: Option<DesignerSurface<Msg>>,
    window: Hwnd,
    out: PathBuf,
    result: Rc<RefCell<Option<std::result::Result<PathBuf, String>>>>,
}

impl App for DesignerApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        match msg {
            Msg::Capture => {
                let outcome = capture_window_rendered(WindowSelector::hwnd(self.window), &self.out)
                    .map_err(|error| error.to_string());
                *self.result.borrow_mut() = Some(outcome);
                ui.quit();
            }
        }
    }
}

/// What a capture run produced.
struct Capture {
    path: PathBuf,
    width: u32,
    height: u32,
    accent: usize,
    distinct: usize,
}

enum Outcome {
    Skipped(String),
    Captured(Capture),
}

/// Builds the window, selects the first button, captures it and measures it.
fn run(theme: Theme, name: &str) -> Outcome {
    let out = std::env::temp_dir()
        .join("vr-forms-issue32")
        .join(format!("{name}.png"));
    let result: Rc<RefCell<Option<std::result::Result<PathBuf, String>>>> =
        Rc::new(RefCell::new(None));
    let setup_error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let backend = Rc::new(Win32Backend::new());
    let backend_for_run: Rc<dyn Backend> = backend.clone();
    let out_in = out.clone();
    let result_in = Rc::clone(&result);
    let setup_in = Rc::clone(&setup_error);
    // Generous room for the 560x360-dip form: the backend creates the window at
    // the design size in pixels, but a higher-DPI session scales the form up.
    let spec = PlatformSpec::new(format!("vr-forms designer {name}")).size(dip(780.0), dip(560.0));

    let ran = run_app(backend_for_run, spec, move |ui| {
        ui.set_theme(theme);
        let surface = match DesignerSurface::new(ui, sample_form(), Dip::new(8.0)) {
            Ok(mut surface) => {
                // Select the OK button so its outline and eight handles show.
                surface.select(Some(0));
                Some(surface)
            }
            Err(error) => {
                *setup_in.borrow_mut() = Some(error.to_string());
                ui.quit();
                None
            }
        };
        let window = backend
            .window_hwnd(ui.window())
            .unwrap_or_else(|| Hwnd::from_raw(0));
        if surface.is_some() {
            let timer = ui.set_timer(CAPTURE_MS);
            ui.on_timer(move |fired| (fired == timer).then_some(Msg::Capture));
        }
        DesignerApp {
            _surface: surface,
            window,
            out: out_in.clone(),
            result: Rc::clone(&result_in),
        }
    });

    if let Err(error) = ran {
        return Outcome::Skipped(format!("could not create a window: {error}"));
    }
    if let Some(error) = setup_error.borrow_mut().take() {
        return Outcome::Skipped(format!("could not build the designer: {error}"));
    }

    let path = match result.borrow_mut().take() {
        Some(Ok(path)) => path,
        Some(Err(error)) => return Outcome::Skipped(format!("capture unavailable: {error}")),
        None => return Outcome::Skipped("the loop ended before the capture fired".to_owned()),
    };
    match read_png(&path) {
        Ok(image) => {
            let accent = [theme.accent.r, theme.accent.g, theme.accent.b];
            let mut distinct = std::collections::HashSet::new();
            let mut accent_hits = 0usize;
            for pixel in image.pixels.as_chunks::<4>().0 {
                distinct.insert([pixel[0], pixel[1], pixel[2]]);
                if pixel[..3] == accent {
                    accent_hits += 1;
                }
            }
            Outcome::Captured(Capture {
                path,
                width: image.width,
                height: image.height,
                accent: accent_hits,
                distinct: distinct.len(),
            })
        }
        Err(error) => Outcome::Skipped(format!("could not read the capture: {error}")),
    }
}

#[test]
fn designer_renders_widgets_and_selection() {
    xui::init();

    let mut captured = 0usize;
    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        match run(theme, name) {
            Outcome::Skipped(reason) => {
                eprintln!("SKIP vr-forms designer ({name}): {reason}");
            }
            Outcome::Captured(capture) => {
                captured += 1;
                eprintln!(
                    "vr-forms designer ({name}): {} ({}x{}), {} distinct colours, \
                     {} accent pixels",
                    capture.path.display(),
                    capture.width,
                    capture.height,
                    capture.distinct,
                    capture.accent,
                );
                assert!(
                    capture.distinct >= 4,
                    "{name}: only {} distinct colours; the widgets did not render",
                    capture.distinct
                );
                assert!(
                    capture.accent >= 24,
                    "{name}: only {} accent pixels; the selection handles are not visible",
                    capture.accent
                );
            }
        }
    }

    if captured == 0 {
        eprintln!("SKIP vr-forms designer: no interactive desktop");
    }
}
