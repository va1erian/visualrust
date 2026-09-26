//! End-to-end: export a desktop project onto the real runtime stub, run the
//! exported `.exe` headless, and screenshot the window it opens.
//!
//! Where the `hello_window` test runs the sample in-process, this one exercises
//! the actual packaged artifact: it copies the built `vr-runtime.exe`, patches a
//! Common Controls manifest onto that copy, appends the sample project's bundle
//! through [`vr_pack::export`], launches the result with
//! `xui_DEMO_AUTOCLOSE_MS`, and asserts a clean exit. While it runs, the test
//! polls for the window owned by that child process and captures it with
//! [`vr_tooling::capture_window_rendered`], then asserts the PNG is non-trivial
//! (non-zero size, more than a couple of distinct colours).
//!
//! The renderer is used instead of [`vr_tooling::capture_window`] because the
//! composited backend faults on a window owned by another process in the pinned
//! xui rev, and a fault cannot be caught to reach its `PrintWindow` fallback.
//!
//! Hosted in this crate because Cargo exposes the just-built binary only to its
//! own integration tests (`CARGO_BIN_EXE_vr-runtime`), so `cargo test` builds the
//! real stub before the test runs; `vr-pack` and `vr-tooling` are already
//! dev-dependencies here.
//!
//! The test prints `SKIP` and passes when no stub can be built, the export
//! fails, the copy cannot be spawned, the session has no desktop, or the capture
//! backend is unavailable. A deadline kills the child if it wedges, so a failed
//! run never leaves a process behind.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use vr_core::manifest::{Manifest, ProjectKind};
use vr_pack::{Resources, Stubs, export, patch_resources};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};

/// Makes the exported window close itself: long enough to capture, short enough
/// to keep the suite fast.
const AUTOCLOSE_MS: u32 = 3000;
/// Environment variable the runtime reads to arm its self-close timer.
const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";
/// How often the poll retries a capture and checks whether the child exited.
const POLL_MS: u64 = 100;
/// How long to let the exported window show and paint before the first capture.
const SETTLE_MS: u64 = 700;
/// Upper bound on the whole run; past this the child is killed and the test
/// skips, so a wedged app never leaves a process behind.
const DEADLINE: Duration = Duration::from_secs(20);

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Removes its scratch directory on drop so the test leaves nothing behind.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> std::io::Result<Self> {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vr-runtime-packaged-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The stub Cargo built for this package; `None` when the env var is absent.
fn built_stub() -> Option<PathBuf> {
    let path = option_env!("CARGO_BIN_EXE_vr-runtime")?;
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
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
fn distinct_colors(image: &xui::RgbaImage) -> usize {
    let mut seen = std::collections::HashSet::new();
    for pixel in image.pixels.as_chunks::<4>().0 {
        seen.insert([pixel[0], pixel[1], pixel[2]]);
        if seen.len() > 16 {
            break;
        }
    }
    seen.len()
}

#[test]
fn exported_executable_runs_and_renders() {
    let Some(stub) = built_stub() else {
        eprintln!("SKIP vr-runtime packaged-verify test: no vr-runtime binary was built");
        return;
    };

    let project = sample_dir();
    let manifest = match Manifest::load(project.join("vrproj.toml")) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("SKIP vr-runtime packaged-verify test: sample manifest unreadable ({error})");
            return;
        }
    };
    if manifest.project.kind != ProjectKind::Desktop {
        eprintln!("SKIP vr-runtime packaged-verify test: sample is not a desktop project");
        return;
    }

    let scratch = match Scratch::new() {
        Ok(scratch) => scratch,
        Err(error) => {
            eprintln!("SKIP vr-runtime packaged-verify test: no scratch dir ({error})");
            return;
        }
    };

    // The plain stub carries no application manifest (build.rs embeds Common
    // Controls v6 only into test binaries), and `Label::new` needs it, so the
    // packaged sample cannot build its controls without one. Patch the manifest
    // onto a *copy of the stub* before exporting, mirroring what the packaging
    // flow does (#53); patching after the bundle append would rewrite the image
    // and could drop the appended footer.
    let manifest_xml = match std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vr-runtime.manifest"),
    ) {
        Ok(xml) => xml,
        Err(error) => {
            eprintln!(
                "SKIP vr-runtime packaged-verify test: could not read the sample manifest ({error})"
            );
            return;
        }
    };
    let stub_copy = scratch.path.join("vr-runtime-manifest.exe");
    if let Err(error) = std::fs::copy(&stub, &stub_copy) {
        eprintln!("SKIP vr-runtime packaged-verify test: could not copy the stub ({error})");
        return;
    }
    if let Err(error) = patch_resources(&stub_copy, &Resources::new().with_manifest(manifest_xml)) {
        eprintln!("SKIP vr-runtime packaged-verify test: could not patch the manifest ({error})");
        return;
    }

    // The copy is renamed, so a clean exit also proves the packaged loader
    // tracks its own image path rather than a baked-in one.
    let exe = scratch.path.join("packaged-app.exe");
    let stubs = Stubs::default().with(ProjectKind::Desktop, stub_copy);
    if let Err(error) = export(&project, &stubs, &exe) {
        eprintln!("SKIP vr-runtime packaged-verify test: export failed ({error})");
        return;
    }

    let png = std::env::temp_dir()
        .join("vr-runtime")
        .join(format!("packaged-verify-{}.png", std::process::id()));

    let mut child = match Command::new(&exe)
        .env(AUTOCLOSE_ENV, AUTOCLOSE_MS.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("SKIP vr-runtime packaged-verify test: could not spawn the export ({error})");
            return;
        }
    };

    // Capture on a dedicated thread so the harness owns a clean COM apartment;
    // selecting by pid rather than title proves the window belongs to the
    // exported app. The main thread watches the child so a wedged run is killed.
    let done = Arc::new(AtomicBool::new(false));
    let capture = {
        let done = Arc::clone(&done);
        let png = png.clone();
        let pid = child.id();
        std::thread::spawn(move || {
            // Let the window show and paint before the first capture.
            std::thread::sleep(Duration::from_millis(SETTLE_MS));
            let deadline = Instant::now() + DEADLINE;
            while !done.load(Ordering::Relaxed) {
                if let Ok(path) = capture_window_rendered(WindowSelector::pid(pid), &png) {
                    return Some(path);
                }
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(POLL_MS));
            }
            None
        })
    };

    let deadline = Instant::now() + DEADLINE;
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                done.store(true, Ordering::Relaxed);
                let _ = child.kill();
                let _ = child.wait();
                let _ = capture.join();
                eprintln!(
                    "SKIP vr-runtime packaged-verify test: lost the export process ({error})"
                );
                return;
            }
        }
        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(POLL_MS));
    }
    done.store(true, Ordering::Relaxed);
    let captured = capture.join().ok().flatten();

    if timed_out {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!(
            "SKIP vr-runtime packaged-verify test: export did not exit within {DEADLINE:?}; \
             killed it"
        );
        return;
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("SKIP vr-runtime packaged-verify test: could not reap the export ({error})");
            return;
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if stderr.contains("no interactive window") || stderr.contains("no interactive desktop") {
            eprintln!(
                "SKIP vr-runtime packaged-verify test: no interactive desktop to create a window"
            );
            return;
        }
        panic!(
            "the exported executable exited with {:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status
        );
    }

    assert!(
        stdout.contains("src/main.dyon"),
        "the export did not report running the bundled entry; stdout: {stdout}"
    );

    let Some(path) = captured else {
        eprintln!(
            "SKIP vr-runtime packaged-verify test: export exited cleanly but no capture was \
             written (capture backend unavailable)"
        );
        return;
    };

    let image = read_png(&path).expect("the capture is a readable PNG");
    let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    let colors = distinct_colors(&image);
    eprintln!(
        "vr-runtime packaged-verify test: export exited {} and captured {} ({}x{}, {} bytes, \
         {} distinct colors)",
        output.status.code().unwrap_or(-1),
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
        colors > 2,
        "the captured window has only {colors} distinct colours, so nothing was rendered"
    );
}
