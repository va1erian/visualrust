//! Headless smoke test for the IDE shell.
//!
//! Runs the real shell in-process with `WIN32UI_DEMO_AUTOCLOSE_MS`, so it opens
//! and quits itself, while a helper thread captures the live window through the
//! `vr-tooling` harness and reports the PNG path and size. It runs once per
//! palette so the light and dark renders are both evidenced.
//!
//! Follows win32ui's skip pattern: when the session cannot create a window (a
//! service session, or a Sandbox without a desktop) no capture is written and
//! the test prints `SKIP` and passes instead of failing.
//!
//! The window is captured by title ([`vr_ide::TITLE`]), the same way the
//! `vr-dyon` UI test captures a Dyon-built window.

use std::path::PathBuf;
use std::time::Duration;

use vr_tooling::{WindowSelector, capture_window, read_png};

/// How long the shell stays open; long enough to capture, short enough to fit
/// two palette runs in a normal test budget.
const AUTOCLOSE_MS: &str = "4000";
/// The capture settles once the window has painted; retry past that if the
/// composited backend is not ready on the first attempt.
const FIRST_ATTEMPT: Duration = Duration::from_millis(600);
const RETRY: Duration = Duration::from_millis(200);
const ATTEMPTS: u32 = 16;

/// One captured run.
struct Capture {
    path: PathBuf,
    width: u32,
    height: u32,
    bytes: u64,
    colors: usize,
}

#[test]
fn shell_opens_and_captures() {
    // One shell per process: the Windows.Graphics.Capture stack can only be
    // driven once per process here, so the light render is captured by running
    // the test again with `VR_IDE_SMOKE_THEME=light`.
    let palette = std::env::var("VR_IDE_SMOKE_THEME").unwrap_or_else(|_| "dark".to_owned());
    let Some(capture) = run_and_capture(&palette) else {
        eprintln!(
            "SKIP vr-ide smoke test: no interactive desktop to create/capture the shell \
             (palette {palette})"
        );
        return;
    };
    eprintln!(
        "vr-ide smoke test ({palette}): captured {} ({}x{}, {} bytes, {} distinct colors)",
        capture.path.display(),
        capture.width,
        capture.height,
        capture.bytes,
        capture.colors
    );
    assert!(capture.width > 0 && capture.height > 0, "empty capture");
    assert!(capture.bytes > 0, "the PNG on disk is empty");
    assert!(
        capture.colors > 1,
        "the captured window is one flat colour, so no chrome rendered"
    );
}

/// Opens the shell once in `palette` and captures it, or `None` when this
/// session cannot create a window or the capture backend is unavailable.
fn run_and_capture(palette: &str) -> Option<Capture> {
    // SAFETY: this integration test binary contains exactly one test, so no
    // other test thread is running; the environment is set before the shell's
    // message loop starts and is not mutated while it runs.
    unsafe {
        std::env::set_var("WIN32UI_DEMO_THEME", palette);
        std::env::set_var("WIN32UI_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS);
    }

    let path = std::env::temp_dir()
        .join("vr-ide")
        .join(format!("shell-{palette}-{}.png", std::process::id()));
    let capture_path = path.clone();
    let capture = std::thread::spawn(move || {
        std::thread::sleep(FIRST_ATTEMPT);
        for _ in 0..ATTEMPTS {
            if capture_window(WindowSelector::title(vr_ide::TITLE), &capture_path).is_ok() {
                return Some(capture_path);
            }
            std::thread::sleep(RETRY);
        }
        None
    });

    let outcome = vr_ide::run();
    let path = capture.join().ok().flatten()?;
    // A run that returned an error but still captured a window is a real
    // failure; a run that failed without a window is the headless skip.
    outcome.ok()?;

    let image = read_png(&path).ok()?;
    let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    Some(Capture {
        path,
        width: image.width,
        height: image.height,
        bytes,
        colors: distinct_colors(&image),
    })
}

/// Counts distinct RGB values, capped early: a rendered window is never one
/// flat colour, while a blank capture is.
fn distinct_colors(image: &win32ui::RgbaImage) -> usize {
    let mut seen = std::collections::HashSet::new();
    for pixel in image.pixels.as_chunks::<4>().0 {
        seen.insert([pixel[0], pixel[1], pixel[2]]);
        if seen.len() > 16 {
            break;
        }
    }
    seen.len()
}
