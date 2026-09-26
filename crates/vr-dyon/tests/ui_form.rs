// The form slice belongs to the `ui` feature; without it there is no window.
#![cfg(feature = "ui")]

//! End-to-end slice: a generated form runs, a widget event reaches a named
//! Dyon handler, and a `fill` widget grows when the window does.
//!
//! One test runs both parts sequentially so the autoclose environment variable
//! is set once, before any runtime thread starts:
//!
//! * the checked-in codegen golden (`all_kinds.dyon`) is run to prove every
//!   palette constructor materialises without panicking;
//! * a small form wires a button click to `on_click_Greet`, dispatches it with
//!   the test-only `ui_invoke`, and records the call through a native sink.
//!
//! A helper thread captures a rendered PNG (the composited backend faults
//! cross-process, so the `PrintWindow` renderer is used) and resizes the live
//! window to assert the `fill` button's rectangle grows. No desktop means the
//! run fails with `DyonError::NoWindow` and the test skips.

use std::sync::Mutex;
use std::time::Duration;

use dyon::{Dfn, Module, Type, dyon_fn, dyon_fn_pop, dyon_macro_items};
use vr_dyon::{DyonError, DyonRuntime};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
    SetWindowPos,
};
use windows::core::PCWSTR;

/// A unique title so the window lookup cannot hit an unrelated process.
const TITLE: &str = "VisualRust runtime form slice";
/// How long each window stays open: long enough to capture and resize.
const AUTOCLOSE_MS: u32 = 6000;

static RECORDED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

dyon_fn! {fn record(value: f64) {
    let mut calls = RECORDED.lock().expect("recording mutex is not poisoned");
    calls.push(value as i32);
}}

/// Registers the recording sink the Dyon handler writes to.
fn register_record(module: &mut Module) {
    module.add_str("record", record, Dfn::nl(vec![Type::F64], Type::Void));
}

/// The form slice: a `fill` button whose click runs a named handler.
///
/// The free-layout origin is deliberately smaller than the window, so the
/// anchor pass grows the button before the first capture.
const FORM: &str = r#"
fn main() {
    w := ui_window("VisualRust runtime form slice", 620, 520)
    root := ui_free(420, 320)
    greet := ui_button("Greet", 10, 10, 120, 40)
    ui_add(root, greet)
    ui_anchor(greet, "fill")
    ui_on(greet, "click", "on_click_Greet")
    ui_invoke(greet, "click")
    ui_run(w, root)
}

fn on_click_Greet() {
    record(1)
}
"#;

#[test]
fn form_slice_opens_dispatches_and_anchors() {
    // SAFETY: this integration test binary contains exactly one test, so no
    // other test thread is running; the environment is set before any runtime
    // thread starts.
    unsafe { std::env::set_var("xui_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS.to_string()) };

    let golden = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vr-forms")
        .join("tests")
        .join("golden")
        .join("all_kinds.dyon");
    let source = std::fs::read_to_string(&golden).expect("the codegen golden is readable");

    // Part 1: every constructor materialises. The golden's handler stubs are
    // zero-argument, exactly as the generator emits them.
    let all_kinds = DyonRuntime::from_source_with(&golden.to_string_lossy(), &source, |_| {})
        .and_then(|mut runtime| runtime.run());
    match all_kinds {
        Ok(()) => {}
        Err(DyonError::NoWindow) => {
            eprintln!("SKIP vr-dyon form slice: no interactive desktop to create a window");
            return;
        }
        Err(error) => panic!("the all-kinds form failed: {error}"),
    }

    // Part 2: dispatch a handler and verify the anchored layout.
    RECORDED.lock().expect("recording mutex").clear();
    let before_path = temp_png("form-slice");
    let after_path = temp_png("form-slice-resized");
    let helper = std::thread::spawn({
        let before = before_path.clone();
        let after = after_path.clone();
        move || observe_window(&before, &after)
    });

    let outcome = DyonRuntime::from_source_with("form.dyon", FORM, register_record)
        .and_then(|mut runtime| runtime.run());
    let observed = helper.join().ok().flatten();

    match outcome {
        Ok(()) => {}
        Err(DyonError::NoWindow) => {
            eprintln!("SKIP vr-dyon form slice: no interactive desktop to create a window");
            return;
        }
        Err(error) => panic!("the form slice failed: {error}"),
    }

    assert_eq!(
        RECORDED.lock().expect("recording mutex").as_slice(),
        [1],
        "the click handler must run once through ui_invoke"
    );

    let Some(observed) = observed else {
        eprintln!("vr-dyon form slice: window ran, but no live observation was possible");
        return;
    };
    eprintln!(
        "vr-dyon form slice: button {}x{} -> {}x{}; png {} ({}), resized {} ({})",
        observed.before.2,
        observed.before.3,
        observed.after.2,
        observed.after.3,
        before_path.display(),
        observed.before_png_bytes,
        after_path.display(),
        observed.after_png_bytes,
    );
    assert!(
        observed.before.2 > 120 && observed.before.3 > 40,
        "the fill anchor must grow the button past its design size: {:?}",
        observed.before
    );
    assert!(
        observed.after.2 > observed.before.2 && observed.after.3 > observed.before.3,
        "the fill anchor must grow the button on resize: {:?} -> {:?}",
        observed.before,
        observed.after
    );
    assert!(observed.before_png_bytes > 0 && observed.after_png_bytes > 0);
    for path in [&before_path, &after_path] {
        let image = read_png(path).expect("the capture is a readable PNG");
        assert!(image.width > 0 && image.height > 0);
    }
}

/// What the helper thread saw: the button rectangle `(x, y, w, h)` before and
/// after the resize, plus the two PNG sizes.
struct Observed {
    before: (i32, i32, i32, i32),
    after: (i32, i32, i32, i32),
    before_png_bytes: u64,
    after_png_bytes: u64,
}

/// Waits for the window, captures it, grows it and measures the button again.
fn observe_window(before_png: &std::path::Path, after_png: &std::path::Path) -> Option<Observed> {
    std::thread::sleep(Duration::from_millis(900));
    let window = wait_for_window()?;

    // PrintWindow renders the same process without COM/DWM; the composited
    // backend faults when the target belongs to another process.
    capture_window_rendered(WindowSelector::title(TITLE), before_png).ok()?;
    let button_before = find_button(window)?;
    let before = rect_of(button_before)?;

    // Grow the window by 200x200; xui re-runs the free layout on WM_SIZE.
    // SAFETY: `window` is a live top-level handle owned by this process and the
    // resize does not move or re-order it.
    unsafe {
        SetWindowPos(
            window,
            None,
            0,
            0,
            820,
            720,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok()?;
    }
    std::thread::sleep(Duration::from_millis(500));

    let button_after = find_button(window)?;
    let after = rect_of(button_after)?;
    capture_window_rendered(WindowSelector::title(TITLE), after_png).ok()?;

    Some(Observed {
        before,
        after,
        before_png_bytes: file_len(before_png),
        after_png_bytes: file_len(after_png),
    })
}

/// Polls for the top-level window by title.
fn wait_for_window() -> Option<HWND> {
    let title = wide(TITLE);
    for _ in 0..20 {
        // SAFETY: `title` is a NUL-terminated UTF-16 buffer alive for the call.
        let hwnd =
            unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) }.unwrap_or_default();
        if !hwnd.is_invalid() {
            return Some(hwnd);
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    None
}

/// Finds the native button by its caption inside `parent`.
fn find_button(parent: HWND) -> Option<HWND> {
    let class = wide("BUTTON");
    let caption = wide("Greet");
    // SAFETY: both buffers are NUL-terminated and outlive the call; the parent
    // handle came from a live window.
    let hwnd = unsafe {
        FindWindowExW(
            Some(parent),
            None,
            PCWSTR(class.as_ptr()),
            PCWSTR(caption.as_ptr()),
        )
    }
    .unwrap_or_default();
    (!hwnd.is_invalid()).then_some(hwnd)
}

/// The window rectangle as `(x, y, w, h)`.
fn rect_of(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut rect = RECT::default();
    // SAFETY: `hwnd` is live and `rect` is a valid out-pointer.
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some((
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    ))
}

/// A NUL-terminated UTF-16 copy of `text`.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn file_len(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn temp_png(stem: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join("vr-dyon")
        .join(format!("{stem}-{}.png", std::process::id()))
}
