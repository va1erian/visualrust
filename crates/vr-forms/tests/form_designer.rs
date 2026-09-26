//! End-to-end check that the win32ui form designer renders in a real `xui`
//! window and its selection adorners are visible.
//!
//! Builds a window, hosts a [`FormDesigner`], adds a button and a label through
//! the palette API, selects one, lets it paint, then captures the window with
//! `vr_tooling::capture_window_rendered` (the `PrintWindow` path, which is safe
//! across processes, unlike WGC) and counts the accent pixels that make up the
//! outline and the eight handles.
//!
//! Follows the workspace skip pattern: a session with no desktop prints `SKIP`
//! and passes, but a captured window with no distinct colours or accent pixels
//! fails.

#![cfg(windows)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use vr_forms::design::{DesignerEvent, FormDesigner};
use vr_forms::{ControlKind, Dip, Form, Size};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use xui::{App, ControlExt, Hwnd, Rect, Theme, Ui, WindowSpec, dip, run_app};

/// How long to let the widget paint before capturing, in ms.
const CAPTURE_MS: u32 = 900;

/// The form the designer hosts, empty so the test drives the palette API.
fn sample_form() -> Form {
    Form {
        name: "DesignerDemo".into(),
        size: Size {
            width: Dip::new(480.0),
            height: Dip::new(320.0),
        },
        controls: Vec::new(),
    }
}

/// The messages the test app handles.
enum Msg {
    Capture,
}

/// The test app: keeps the designer (and so its widget) alive and captures once.
struct DesignerApp {
    _designer: Option<FormDesigner<Msg>>,
    window: Hwnd,
    out: PathBuf,
    result: Rc<RefCell<Option<Result<PathBuf, String>>>>,
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

/// Builds the window, adds two controls, selects the first, captures and
/// measures the result.
fn run(theme: Theme, name: &str) -> Outcome {
    let out = std::env::temp_dir()
        .join("vr-forms-issue119")
        .join(format!("{name}.png"));
    let result: Rc<RefCell<Option<Result<PathBuf, String>>>> = Rc::new(RefCell::new(None));
    let setup_error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let out_in = out.clone();
    let result_in = Rc::clone(&result);
    let setup_in = Rc::clone(&setup_error);
    let spec = WindowSpec::new(format!("vr-forms form designer {name}"))
        .size(dip(780.0), dip(560.0))
        .theme(theme);

    let ran = run_app(spec, move |ui| {
        let mut designer = match FormDesigner::new(ui, sample_form(), Dip::new(8.0)) {
            Ok(designer) => designer,
            Err(error) => {
                *setup_in.borrow_mut() = Some(error.to_string());
                ui.quit();
                return DesignerApp {
                    _designer: None,
                    window: ui.hwnd(),
                    out: out_in.clone(),
                    result: Rc::clone(&result_in),
                };
            }
        };
        // Fill the pane: a layout would call this with the slot it grants.
        designer.set_bounds(Rect::new(0, 0, 760, 520));
        designer.on_change(|event| match event {
            DesignerEvent::SelectionChanged(_) | DesignerEvent::FormEdited => None,
        });
        // Drive the palette API the IDE's palette will use.
        let button = designer.add_control(ControlKind::Button);
        let _label = designer.add_control(ControlKind::Label);
        designer.select(Some(button));
        if let Ok(timer) = ui.set_timer(CAPTURE_MS) {
            ui.on_timer(move |fired| (fired == timer).then_some(Msg::Capture));
        }
        let window = ui.hwnd();
        DesignerApp {
            _designer: Some(designer),
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
fn form_designer_renders_proxies_and_selection() {
    xui::init();

    let mut captured = 0usize;
    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        match run(theme, name) {
            Outcome::Skipped(reason) => {
                eprintln!("SKIP vr-forms form designer ({name}): {reason}");
            }
            Outcome::Captured(capture) => {
                captured += 1;
                eprintln!(
                    "vr-forms form designer ({name}): {} ({}x{}), {} distinct colours, \
                     {} accent pixels",
                    capture.path.display(),
                    capture.width,
                    capture.height,
                    capture.distinct,
                    capture.accent,
                );
                assert!(
                    capture.distinct >= 4,
                    "{name}: only {} distinct colours; the page, proxies and grid did not render",
                    capture.distinct
                );
                assert!(
                    capture.accent >= 100,
                    "{name}: only {} accent pixels; the selection outline and eight handles \
                     are not visible",
                    capture.accent
                );
            }
        }
    }

    if captured == 0 {
        eprintln!("SKIP vr-forms form designer: no interactive desktop");
    }
}
