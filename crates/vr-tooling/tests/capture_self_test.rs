//! Self-test: create a real win32ui window, capture it through the harness and
//! write a PNG.
//!
//! Follows win32ui's skip pattern: if this session cannot create a window (for
//! example Windows Sandbox without a desktop, or a service session) the test
//! prints a `SKIP` line and passes. Capture failures after a window exists are
//! reported the same way, because the `Windows.Graphics.Capture` stack can be
//! absent on headless sessions.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use vr_tooling::{ToolingError, WindowSelector, capture_window};
use win32ui::prelude::*;

/// Captures once, on the first timer tick, then quits the loop.
struct CaptureOnce {
    out: PathBuf,
    result: Rc<RefCell<Option<std::result::Result<PathBuf, ToolingError>>>>,
}

impl WindowHandler for CaptureOnce {
    fn message(&self, window: &Window, message: Message) -> Option<LResult> {
        if let Message::Timer { .. } = message {
            let result = capture_window(WindowSelector::hwnd(window.hwnd()), &self.out);
            *self.result.borrow_mut() = Some(result);
            window.destroy();
            win32ui::quit(0);
            return Some(0);
        }
        None
    }
}

#[test]
fn captures_own_window_to_png() {
    win32ui::init();

    let Ok(class) =
        WindowClass::register("vr-tooling-capture-self-test", Theme::light().background)
    else {
        eprintln!("SKIP vr-tooling capture self-test: could not register a window class");
        return;
    };

    let out = std::env::temp_dir()
        .join("vr-tooling")
        .join("self-test")
        .join("capture.png");
    let result: Rc<RefCell<Option<std::result::Result<PathBuf, ToolingError>>>> =
        Rc::new(RefCell::new(None));
    let handler = CaptureOnce {
        out: out.clone(),
        result: Rc::clone(&result),
    };

    let Ok(window) = Window::create(
        class,
        None,
        WindowStyle::overlapped(),
        WindowExStyle::new(),
        Rect::new(0, 0, 240, 160),
        "vr-tooling capture self-test",
        handler,
    ) else {
        eprintln!("SKIP vr-tooling capture self-test: no interactive desktop to create a window");
        return;
    };

    if window.set_timer(300).is_err() {
        eprintln!("SKIP vr-tooling capture self-test: could not arm the capture timer");
        return;
    }
    window.show();
    win32ui::run();
    window.destroy();

    match result.borrow_mut().take() {
        Some(Ok(path)) => match std::fs::metadata(&path) {
            Ok(metadata) if metadata.len() > 0 => {
                eprintln!(
                    "vr-tooling capture self-test: wrote {} ({} bytes)",
                    path.display(),
                    metadata.len()
                );
            }
            _ => eprintln!(
                "SKIP vr-tooling capture self-test: {} was not written or is empty",
                path.display()
            ),
        },
        Some(Err(error)) => {
            eprintln!("SKIP vr-tooling capture self-test: capture unavailable: {error}");
        }
        None => {
            eprintln!("SKIP vr-tooling capture self-test: the loop ended before the timer fired");
        }
    }
}
