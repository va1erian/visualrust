// The window test belongs to the `ui` feature; without it there are no `ui_*`
// bindings to exercise.
#![cfg(feature = "ui")]

//! Integration test: a complete window built entirely from a Dyon script.
//!
//! The script calls `ui_window` / `ui_column` / `ui_label` / `ui_run`, so this
//! exercises the plan objects round-tripping through the Dyon runtime and the
//! widget tree being materialised inside `ui_run`. The window closes through
//! `WIN32UI_DEMO_AUTOCLOSE_MS`, and a helper thread captures it with the
//! `vr-tooling` harness so the test can inspect real pixels.
//!
//! Follows win32ui's skip pattern: when this session cannot create a window the
//! test prints `SKIP` and passes instead of failing.

use std::time::Duration;

use vr_dyon::{DyonError, DyonRuntime};
use vr_tooling::{WindowSelector, capture_window, read_png};

/// Unique enough that the window lookup cannot hit an unrelated process.
const TITLE: &str = "VisualRust Dyon ui smoke";

/// A window with a nested layout, built only from Dyon.
const SCRIPT: &str = r#"
fn main() {
    win := ui_window("VisualRust Dyon ui smoke", 520, 320)
    root := ui_column([
        ui_label("Hello from Dyon"),
        ui_row([
            ui_label("left"),
            ui_label("right")
        ]),
        ui_column([
            ui_label("nested")
        ])
    ])
    ui_run(win, root)
}
"#;

/// How long the window stays open; long enough to capture it, short enough to
/// keep the suite fast.
const AUTOCLOSE_MS: u32 = 2500;

#[test]
fn dyon_script_builds_and_runs_a_window() {
    // SAFETY: this integration test binary contains exactly one test, so no
    // other test thread is running; the process environment is set once before
    // the runtime thread starts.
    unsafe { std::env::set_var("WIN32UI_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS.to_string()) };

    let capture_path = std::env::temp_dir()
        .join("vr-dyon")
        .join(format!("ui-window-{}.png", std::process::id()));
    let path_for_thread = capture_path.clone();
    let capture = std::thread::spawn(move || {
        // Give the message loop time to show and paint the window, then keep
        // retrying until the composited backend produces a frame.
        std::thread::sleep(Duration::from_millis(500));
        for _ in 0..10 {
            if capture_window(WindowSelector::title(TITLE), &path_for_thread).is_ok() {
                return Some(path_for_thread);
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        None
    });

    let outcome = DyonRuntime::from_source("ui_window.dyon", SCRIPT).and_then(|mut rt| rt.run());

    let captured = capture.join().ok().flatten();

    match outcome {
        Ok(()) => {}
        Err(DyonError::NoWindow) => {
            eprintln!("SKIP vr-dyon ui window test: no interactive desktop to create a window");
            return;
        }
        Err(error) => panic!("the Dyon UI program failed: {error}"),
    }

    let Some(path) = captured else {
        eprintln!(
            "vr-dyon ui window test: window ran, but no capture was written (capture backend \
             unavailable)"
        );
        return;
    };

    let image = read_png(&path).expect("the capture is a readable PNG");
    let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    let colors = distinct_colors(&image);
    eprintln!(
        "vr-dyon ui window test: captured {} ({}x{}, {} bytes, {} distinct colors)",
        path.display(),
        image.width,
        image.height,
        bytes,
        colors
    );
    assert!(
        image.width > 0 && image.height > 0,
        "the capture is not empty"
    );
    assert!(bytes > 0, "the PNG on disk is empty");
    assert!(
        colors > 1,
        "the captured window is a single flat colour, so nothing was rendered"
    );
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
