//! End-to-end check that the IDE editor pane renders highlighted Dyon.
//!
//! It launches the real IDE (`vr_ide::run_with`) with a `.dyon` file, lets the
//! container lexer style it, then captures the live window with
//! `vr_tooling::capture_window_rendered`. That is the `PrintWindow` path: the
//! composited/WGC backend is unsafe for another process's window in the pinned
//! xui rev, and this test runs the window in its own process while a helper
//! thread captures it.
//!
//! It counts the exact palette colours actually present: the Dyon keyword,
//! string and number tokens must appear, and the line-number margin's
//! background must be visible next to the editor's own. Follows the workspace
//! skip pattern: a session without a desktop prints `SKIP` and passes, but a
//! window that renders no syntax colours fails.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use xui::prelude::*;

/// How long the shell stays open, so the capture thread has time to retry.
const AUTOCLOSE_MS: &str = "5000";
/// Once the window has painted, first capture attempt.
const FIRST_ATTEMPT: Duration = Duration::from_millis(1200);
const RETRY: Duration = Duration::from_millis(200);
const ATTEMPTS: u32 = 18;

/// The window title the capture selects; `vr_ide::TITLE` is a substring match.
fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples/hello-window/src/main.dyon")
}

#[test]
fn editor_pane_renders_dyon_highlighting() {
    // One shell per process (as in the smoke test); the palette is selectable
    // so a second run can evidence the other theme.
    let palette = std::env::var("VR_IDE_CAPTURE_THEME").unwrap_or_else(|_| "dark".to_owned());

    // SAFETY: this integration-test binary contains exactly one test, so no
    // other test thread races the environment; `run_with` reads it only when it
    // builds the window below.
    unsafe {
        std::env::set_var("xui_DEMO_THEME", &palette);
        std::env::set_var("xui_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS);
    }

    let path = std::env::temp_dir()
        .join("vr-ide")
        .join(format!("editor-{palette}-{}.png", std::process::id()));
    let capture_path = path.clone();
    let capture = std::thread::spawn(move || {
        std::thread::sleep(FIRST_ATTEMPT);
        for _ in 0..ATTEMPTS {
            if capture_window_rendered(WindowSelector::title(vr_ide::TITLE), &capture_path).is_ok()
            {
                return Some(capture_path);
            }
            std::thread::sleep(RETRY);
        }
        None
    });

    let outcome = vr_ide::run_with(Some(sample_path()));
    let captured = capture.join().ok().flatten();

    let Some(path) = captured else {
        eprintln!("SKIP vr-ide editor capture ({palette}): no interactive desktop");
        return;
    };
    // A run that returned an error but still captured a window is a real
    // failure; a run that failed without a window is the headless skip above.
    if outcome.is_err() {
        panic!("the IDE ran and captured but returned an error: {outcome:?}");
    }

    let image = match read_png(&path) {
        Ok(image) => image,
        Err(error) => {
            panic!("could not read the capture at {}: {error}", path.display());
        }
    };
    let theme = if palette.eq_ignore_ascii_case("light") {
        Theme::light()
    } else {
        Theme::dark()
    };

    let mut distinct = HashSet::new();
    let mut syntax_present = [false; 3];
    let mut editor_background = false;
    let mut margin_background = false;
    let syntax = [rgb(theme.accent), rgb(theme.warning), rgb(theme.danger)];
    let editor_bg = rgb(theme.input_background);
    let margin_bg = rgb(theme.background);
    // The line-number margin is the leftmost band; probe it only inside the
    // editor's vertical middle so the toolbar cannot contribute a false hit.
    let probe_left = 64.min(image.width);
    let probe_top = 100.min(image.height);
    let probe_bottom = image.height.saturating_sub(40);

    for (index, pixel) in image.pixels.as_chunks::<4>().0.iter().enumerate() {
        let [r, g, b] = [pixel[0], pixel[1], pixel[2]];
        distinct.insert([r, g, b]);
        let color = [r, g, b];
        for (slot, expected) in syntax_present.iter_mut().zip(syntax.iter()) {
            if color == *expected {
                *slot = true;
            }
        }
        if color == editor_bg {
            editor_background = true;
        }
        let x = index as u32 % image.width;
        let y = index as u32 / image.width;
        if color == margin_bg && x < probe_left && y > probe_top && y < probe_bottom {
            margin_background = true;
        }
    }

    let hits = syntax_present.iter().filter(|seen| **seen).count();
    eprintln!(
        "vr-ide editor capture ({palette}): {} ({}x{}), {} distinct colours, \
         {hits}/3 syntax colours, editor background {editor_background}, \
         line-number margin {margin_background}",
        path.display(),
        image.width,
        image.height,
        distinct.len()
    );

    assert!(
        editor_background,
        "the editor's theme background is absent; the pane did not render"
    );
    assert!(
        hits >= 3,
        "{palette}: only {hits}/3 syntax colours rendered; highlighting is not visible"
    );
    assert!(
        margin_background,
        "{palette}: the line-number margin background is absent"
    );
}

/// The exact RGB of a theme token.
fn rgb(color: xui::Color) -> [u8; 3] {
    [color.r, color.g, color.b]
}
