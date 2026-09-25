//! End-to-end smoke test: the `examples/hello-window` project boots headless.
//!
//! The test reads the sample's `vrproj.toml` through `vr-core` (so the manifest
//! stays the single source of truth for the entry point), runs it through the
//! real [`RuntimeApp`] host with `WIN32UI_DEMO_AUTOCLOSE_MS`, and asserts the
//! run returns `Ok(())` — a clean exit. A helper thread captures the live
//! window through `vr-tooling` so the test reports the PNG path and size.
//!
//! Follows win32ui's skip pattern: when the session cannot create a window the
//! run fails with `DyonError::NoWindow`, the test prints `SKIP` and passes
//! instead of failing.

use std::path::PathBuf;
use std::time::Duration;

use vr_core::manifest::{Manifest, ProjectKind};
use vr_dyon::DyonError;
use vr_runtime::RuntimeApp;
use vr_tooling::{WindowSelector, capture_window, read_png};

/// The sample window title, used to find the window for the capture.
const TITLE: &str = "VisualRust hello window";
/// How long the sample stays open: long enough to capture, short enough to keep
/// the suite fast.
const AUTOCLOSE_MS: u32 = 2500;

#[test]
fn hello_window_sample_runs_headless() {
    let project = sample_dir();
    let manifest = Manifest::load(project.join("vrproj.toml")).expect("the sample manifest loads");
    assert_eq!(
        manifest.project.kind,
        ProjectKind::Desktop,
        "the sample is a desktop project"
    );
    let entry = project.join(&manifest.project.entry);
    let source = std::fs::read_to_string(&entry).expect("the sample entry point is readable");

    // SAFETY: this integration test binary contains exactly one test, so no
    // other test thread is running; the environment is set before the runtime
    // message loop starts and is not mutated afterwards.
    unsafe { std::env::set_var("WIN32UI_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS.to_string()) };

    let capture_path = std::env::temp_dir()
        .join("vr-runtime")
        .join(format!("hello-window-{}.png", std::process::id()));
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

    let outcome = RuntimeApp::from_source(&entry.to_string_lossy(), &source)
        .and_then(|mut app| app.run_main());
    let captured = capture.join().ok().flatten();

    match outcome {
        Ok(()) => {}
        Err(DyonError::NoWindow) => {
            eprintln!(
                "SKIP vr-runtime hello-window test: no interactive desktop to create a window"
            );
            return;
        }
        Err(error) => panic!("the hello-window sample did not exit cleanly: {error}"),
    }

    let Some(path) = captured else {
        eprintln!(
            "vr-runtime hello-window test: sample exited cleanly, but no capture was written \
             (capture backend unavailable)"
        );
        return;
    };

    let image = read_png(&path).expect("the capture is a readable PNG");
    let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    eprintln!(
        "vr-runtime hello-window test: captured {} ({}x{}, {} bytes, {} distinct colors)",
        path.display(),
        image.width,
        image.height,
        bytes,
        distinct_colors(&image)
    );
    assert!(
        image.width > 0 && image.height > 0,
        "the capture is not empty"
    );
    assert!(bytes > 0, "the PNG on disk is empty");
    assert!(
        distinct_colors(&image) > 1,
        "the captured window is one flat colour, so nothing was rendered"
    );
}

/// The `examples/hello-window` directory, anchored to this crate's manifest.
fn sample_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("hello-window")
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
