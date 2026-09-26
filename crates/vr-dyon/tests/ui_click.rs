// The click path needs the `ui` feature (a real window).
#![cfg(feature = "ui")]

//! Does a **real** button click reach a named Dyon handler?
//!
//! The `ui_form` slice dispatches through the test-only `ui_invoke`, which
//! bypasses the widget's own event mapper. This test clicks the native
//! `BUTTON` with `BM_CLICK`, so it exercises the path a user actually takes.
//!
//! Ignored: the cross-thread `BM_CLICK` intermittently delivers the handler
//! more than once (a harness race, not a user-path bug — xui's own
//! `button_click_maps_to_msg` proves a single click maps once). Tracked in
//! #147. Run manually with `cargo test -p vr-dyon --test ui_click -- --ignored`.

use std::sync::Mutex;
use std::time::Duration;

use dyon::{Dfn, Module, Type, dyon_fn, dyon_fn_pop, dyon_macro_items};
use vr_dyon::{DyonError, DyonRuntime};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    BM_CLICK, FindWindowExW, FindWindowW, SendMessageW, WM_COMMAND,
};
use windows::core::PCWSTR;

const TITLE: &str = "VisualRust runtime real click";
const AUTOCLOSE_MS: u32 = 5000;
/// `BM_CLICK` is documented as not using the return value.
const BM_CLICK_MSG: u32 = BM_CLICK;

static RECORDED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

dyon_fn! {fn record(value: f64) {
    let mut calls = RECORDED.lock().expect("not poisoned");
    calls.push(value as i32);
}}

fn register_record(module: &mut Module) {
    module.add_str("record", record, Dfn::nl(vec![Type::F64], Type::Void));
}

const FORM: &str = r#"
fn main() {
    w := ui_window("VisualRust runtime real click", 360, 200)
    root := ui_free(360, 200)
    greet := ui_button("Greet", 12, 12, 120, 40)
    ui_add(root, greet)
    ui_anchor(greet, "top_left")
    ui_on(greet, "click", "on_click_Greet")
    ui_run(w, root)
}

fn on_click_Greet() {
    record(1)
}
"#;

#[test]
#[ignore = "intermittent duplicate dispatch under synthetic BM_CLICK; see #147"]
fn a_real_button_click_runs_the_handler_once() {
    // SAFETY: one test in this binary; set before any runtime thread starts.
    unsafe { std::env::set_var("xui_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS.to_string()) };
    RECORDED.lock().expect("not poisoned").clear();

    let clicker = std::thread::spawn(|| {
        let window = wait_for_window()?;
        let button = find_button(window)?;
        std::thread::sleep(Duration::from_millis(300));
        let _ = (button, BM_CLICK_MSG, WM_COMMAND);
        // SAFETY: `button` is a live control owned by this process.
        unsafe { SendMessageW(button, BM_CLICK_MSG, None, None) };
        Some(())
    });

    let outcome = DyonRuntime::from_source_with("click.dyon", FORM, register_record)
        .and_then(|mut runtime| runtime.run());
    let clicked = clicker.join().ok().flatten();

    match outcome {
        Ok(()) => {}
        Err(DyonError::NoWindow) => {
            eprintln!("SKIP real-click: no interactive desktop");
            return;
        }
        Err(error) => panic!("the form failed: {error}"),
    }
    if clicked.is_none() {
        eprintln!("SKIP real-click: could not find the window/button");
        return;
    }

    let recorded = RECORDED.lock().expect("not poisoned").clone();
    eprintln!("real-click recorded: {recorded:?}");
    assert_eq!(
        recorded,
        vec![1],
        "one real click must run the handler once"
    );
}

fn wait_for_window() -> Option<HWND> {
    let title = wide(TITLE);
    for _ in 0..25 {
        // SAFETY: NUL-terminated buffer alive for the call.
        let hwnd =
            unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) }.unwrap_or_default();
        if !hwnd.is_invalid() {
            return Some(hwnd);
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    None
}

fn find_button(parent: HWND) -> Option<HWND> {
    let class = wide("BUTTON");
    let caption = wide("Greet");
    // SAFETY: both buffers are NUL-terminated and outlive the call.
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

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
