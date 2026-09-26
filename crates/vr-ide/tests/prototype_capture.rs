//! End-to-end capture of the IDE prototype: explorer | central | output.
//!
//! Launches the real shell (no CLI argument, so it falls back to the built-in
//! sample project), captures the live window with
//! `vr_tooling::capture_window_rendered` (the `PrintWindow` path, safe across
//! processes), and probes three regions for the theme colours each pane draws:
//! the explorer's background on the left, the central edit background, and the
//! output edit background at the bottom.
//!
//! Set `VR_IDE_CAPTURE_DESIGN=1` to start in design mode and capture the form
//! pane instead of the editor. Set `VR_IDE_CAPTURE_THEME=light` for the light
//! palette. A session without a desktop prints `SKIP` and passes.

use std::time::Duration;

use vr_tooling::{WindowSelector, capture_window_rendered, read_png};

/// How long the shell stays open, so the capture thread has time to retry.
const AUTOCLOSE_MS: &str = "5000";
/// Once the window has painted, first capture attempt.
const FIRST_ATTEMPT: Duration = Duration::from_millis(1200);
const RETRY: Duration = Duration::from_millis(200);
const ATTEMPTS: u32 = 18;

#[test]
fn prototype_renders_explorer_central_and_output() {
    let palette = std::env::var("VR_IDE_CAPTURE_THEME").unwrap_or_else(|_| "dark".to_owned());
    let design = std::env::var("VR_IDE_CAPTURE_DESIGN").is_ok_and(|value| value == "1");

    // SAFETY: this integration-test binary contains exactly one test, so no
    // other test thread races the environment, and the shell reads these only
    // when it builds the window below.
    unsafe {
        std::env::set_var("xui_DEMO_THEME", &palette);
        std::env::set_var("xui_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS);
        if design {
            std::env::set_var("VR_IDE_PROTOTYPE_DESIGN", "1");
        } else {
            std::env::remove_var("VR_IDE_PROTOTYPE_DESIGN");
        }
    }

    let mode = if design { "design" } else { "editor" };
    let path = std::env::temp_dir().join("vr-ide").join(format!(
        "prototype-{palette}-{mode}-{}.png",
        std::process::id()
    ));
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

    let outcome = vr_ide::run_with(None);
    let captured = capture.join().ok().flatten();

    let Some(path) = captured else {
        eprintln!("SKIP vr-ide prototype capture ({palette}/{mode}): no interactive desktop");
        return;
    };
    if outcome.is_err() {
        panic!("the IDE ran and captured but returned an error: {outcome:?}");
    }

    let image = match read_png(&path) {
        Ok(image) => image,
        Err(error) => panic!("could not read the capture at {}: {error}", path.display()),
    };
    let theme = if palette.eq_ignore_ascii_case("light") {
        xui::Theme::light()
    } else {
        xui::Theme::dark()
    };
    // A read-only edit (the output and design panes) answers `WM_CTLCOLORSTATIC`,
    // so it paints the window background; the Scintilla editor paints its own
    // `input_background`. The two centre widgets are therefore told apart by
    // colour: the editor by `input_background`, the design pane by `background`.
    let background = rgb(theme.background);
    let edit_background = rgb(theme.input_background);
    let central_colour = if design { background } else { edit_background };

    let mut explorer = false;
    let mut central = false;
    let mut output = false;
    let mut distinct = std::collections::HashSet::new();
    for (index, pixel) in image.pixels.as_chunks::<4>().0.iter().enumerate() {
        let color = [pixel[0], pixel[1], pixel[2]];
        distinct.insert(color);
        let x = index as u32 % image.width;
        let y = index as u32 / image.width;
        let fx = x as f64 / image.width.max(1) as f64;
        let fy = y as f64 / image.height.max(1) as f64;
        if color == background && fx < 0.15 && (0.2..0.7).contains(&fy) {
            explorer = true;
        }
        if color == central_colour && (0.35..0.9).contains(&fx) && (0.2..0.55).contains(&fy) {
            central = true;
        }
        if color == background && (0.35..0.9).contains(&fx) && (0.8..0.93).contains(&fy) {
            output = true;
        }
    }

    eprintln!(
        "vr-ide prototype ({palette}/{mode}): {} ({}x{}), {} distinct colours; \
         explorer={explorer}, central={central}, output={output}",
        path.display(),
        image.width,
        image.height,
        distinct.len(),
    );

    assert!(
        explorer,
        "the explorer pane's background is absent on the left"
    );
    assert!(central, "the central pane's edit background is absent");
    assert!(
        output,
        "the output pane's edit background is absent at the bottom"
    );
}

/// The exact RGB of a theme token.
fn rgb(color: xui::Color) -> [u8; 3] {
    [color.r, color.g, color.b]
}
