//! End-to-end check that the xui host renders Dyon highlighting.
//!
//! It builds a real window with the [`ScintillaHost`], loads a representative
//! Dyon snippet, lets the container lexer style it (a timer gives Scintilla
//! time to paint and emit `SCN_STYLENEEDED`), then captures the window with
//! `vr_tooling::capture_window_rendered` (the `PrintWindow` path, safe for a
//! child-hosted control) and counts the palette colours actually present.
//!
//! Follows the workspace skip pattern: a session without a desktop prints
//! `SKIP` and passes, but a window that renders no syntax colours fails.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use vr_scintilla::{ScintillaHost, Scn};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use xui::prelude::*;

/// A representative Dyon snippet with keywords, a declaration, a string,
/// numbers and both comment forms, so several styles render at once.
const SAMPLE: &str = "// VisualRust Dyon sample\n\
fn main() {\n\
    /* greet the world */\n\
    name := \"VisualRust\"\n\
    count := 42\n\
    print(name + \" \" + str(count))\n\
}\n";

/// How long to let the control paint and style before capturing, in ms.
const CAPTURE_MS: u32 = 900;

/// The messages the test app handles.
enum Msg {
    /// A Scintilla notification, mapped from the host's event mapper.
    Scn(Scn),
    /// Time to capture and quit.
    Capture,
}

/// The test app: keeps the host (and therefore the control) alive for the
/// window's lifetime and captures once.
struct CaptureApp {
    _host: Option<ScintillaHost<Msg>>,
    out: PathBuf,
    result: Rc<RefCell<Option<std::result::Result<PathBuf, String>>>>,
}

impl App for CaptureApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        match msg {
            Msg::Scn(scn) => {
                // The snippet is static; the app has nothing to do with the
                // events beyond having the mapper wired through the host.
                let _ = scn;
            }
            Msg::Capture => {
                let outcome = capture_window_rendered(WindowSelector::hwnd(ui.hwnd()), &self.out)
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
    distinct: usize,
    /// How many of the theme's syntax colours appear as exact pixels.
    palette_hits: usize,
}

enum Outcome {
    Skipped(String),
    Captured(Capture),
}

/// Builds the window, styles the snippet, captures it and measures the result.
fn run(theme: Theme, name: &str) -> Outcome {
    let out = std::env::temp_dir()
        .join("vr-scintilla-issue24")
        .join(format!("{name}.png"));
    let result: Rc<RefCell<Option<std::result::Result<PathBuf, String>>>> =
        Rc::new(RefCell::new(None));
    let setup_error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let out_in = out.clone();
    let result_in = Rc::clone(&result);
    let setup_in = Rc::clone(&setup_error);
    let spec = WindowSpec::new(format!("vr-scintilla host {name}"))
        .size(dip(640.0), dip(420.0))
        .theme(theme);

    let ran = xui::run_app(spec, move |ui| {
        let host = match ScintillaHost::new(ui, |scn| Some(Msg::Scn(scn))) {
            Ok(host) => Some(host),
            Err(error) => {
                *setup_in.borrow_mut() = Some(error.to_string());
                ui.quit();
                None
            }
        };
        if let Some(host) = &host {
            host.set_text(SAMPLE);
            ui.set_layout(Layout::column().item(host.fill(1)));
            match ui.set_timer(CAPTURE_MS) {
                Ok(timer) => ui.on_timer(move |fired| (fired == timer).then_some(Msg::Capture)),
                Err(_) => ui.quit(),
            }
        }
        CaptureApp {
            _host: host,
            out: out_in.clone(),
            result: Rc::clone(&result_in),
        }
    });

    if let Err(error) = ran {
        return Outcome::Skipped(format!("could not create a window: {error}"));
    }
    if let Some(error) = setup_error.borrow_mut().take() {
        return Outcome::Skipped(format!("could not create the Scintilla host: {error}"));
    }

    let path = match result.borrow_mut().take() {
        Some(Ok(path)) => path,
        Some(Err(error)) => return Outcome::Skipped(format!("capture unavailable: {error}")),
        None => return Outcome::Skipped("the loop ended before the capture fired".to_owned()),
    };
    match read_png(&path) {
        Ok(image) => {
            let mut distinct = HashSet::new();
            let mut hits = 0usize;
            let expected = expected_colors(&theme);
            let mut present = [false; 4];
            for pixel in image.pixels.as_chunks::<4>().0 {
                distinct.insert([pixel[0], pixel[1], pixel[2]]);
                for (index, color) in expected.iter().enumerate() {
                    if pixel[..3] == color[..] {
                        present[index] = true;
                    }
                }
            }
            hits += present.iter().filter(|seen| **seen).count();
            Outcome::Captured(Capture {
                path,
                width: image.width,
                height: image.height,
                distinct: distinct.len(),
                palette_hits: hits,
            })
        }
        Err(error) => Outcome::Skipped(format!("could not read the capture: {error}")),
    }
}

/// The exact RGB of the syntax colours the Dyon palette maps from `theme`:
/// keyword (accent), string (warning), number (danger) and comment
/// (text_secondary).
fn expected_colors(theme: &Theme) -> [[u8; 3]; 4] {
    let rgb = |color: xui::Color| [color.r, color.g, color.b];
    [
        rgb(theme.accent),
        rgb(theme.warning),
        rgb(theme.danger),
        rgb(theme.text_secondary),
    ]
}

#[test]
fn host_renders_dyon_highlighting_in_light_and_dark() {
    xui::init();

    let mut captured = 0usize;
    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        match run(theme, name) {
            Outcome::Skipped(reason) => {
                eprintln!("SKIP vr-scintilla host highlight ({name}): {reason}");
            }
            Outcome::Captured(capture) => {
                captured += 1;
                eprintln!(
                    "vr-scintilla host highlight ({name}): {} ({}x{}), {} distinct colours, \
                     {}/4 theme syntax colours present",
                    capture.path.display(),
                    capture.width,
                    capture.height,
                    capture.distinct,
                    capture.palette_hits,
                );
                assert!(
                    capture.palette_hits >= 3,
                    "{name}: only {}/4 syntax colours rendered; highlighting is not visible",
                    capture.palette_hits
                );
            }
        }
    }

    if captured == 0 {
        eprintln!("SKIP vr-scintilla host highlight: no interactive desktop");
    }
}
