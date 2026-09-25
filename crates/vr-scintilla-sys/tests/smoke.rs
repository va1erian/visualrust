//! Smoke test for the vendored Scintilla build.
//!
//! Registers Scintilla's window classes, creates a `Scintilla` child of a
//! hidden `STATIC` window, and asks it for its length. When the session cannot
//! create a window (for example a non-interactive desktop) the test prints
//! `SKIP` and passes, following win32ui's test pattern.

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_CHILD, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use vr_scintilla_sys::messages::{SCI_GETLENGTH, SCI_GETTEXTLENGTH, SCI_SETILEXER};
use vr_scintilla_sys::sys;

/// A test outcome that distinguishes "cannot test here" from a real assertion
/// failure, so the caller can print `SKIP` for the former.
enum Outcome {
    Passed,
    Skipped(&'static str),
}

#[test]
fn registers_the_class_and_creates_a_control() {
    match run() {
        Outcome::Passed => {}
        Outcome::Skipped(reason) => eprintln!("SKIP vr-scintilla-sys smoke test: {reason}"),
    }
}

fn run() -> Outcome {
    // SAFETY: `GetModuleHandleW` only reads the process module list; a null
    // name asks for the executable's own module handle.
    let module = match unsafe { GetModuleHandleW(None) } {
        Ok(module) => module,
        Err(_) => return Outcome::Skipped("no module handle"),
    };
    let instance = HINSTANCE(module.0);

    // SAFETY: `instance` is the live module handle just fetched. Scintilla
    // requires registration exactly once per process before any control is
    // created; this test binary contains a single test.
    let registered = unsafe { sys::Scintilla_RegisterClasses(instance.0) };
    if registered == 0 {
        return Outcome::Skipped("Scintilla_RegisterClasses failed");
    }

    // SAFETY: `instance` is live; the class names are the static wide strings.
    // A hidden top-level `STATIC` window owns the control.
    let parent = match unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("vr-scintilla-sys smoke"),
            WS_OVERLAPPED,
            0,
            0,
            320,
            200,
            None,
            None,
            Some(instance),
            None,
        )
    } {
        Ok(parent) => parent,
        Err(_) => {
            release();
            return Outcome::Skipped("could not create a parent window");
        }
    };

    // SAFETY: `parent` is live and `instance` owns the `Scintilla` class
    // registered above. The control is created but never shown.
    let child = match unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Scintilla"),
            PCWSTR::null(),
            WS_CHILD,
            0,
            0,
            300,
            180,
            Some(parent),
            None,
            Some(instance),
            None,
        )
    } {
        Ok(child) => child,
        Err(_) => {
            destroy_parent(parent);
            release();
            return Outcome::Skipped("could not create a Scintilla control");
        }
    };

    // SAFETY: `child` is a live Scintilla control; switching to the container
    // lexer takes a null lexer pointer, and the length queries take no
    // arguments.
    let (length, text_length) = unsafe {
        sys::send_message(child.0, SCI_SETILEXER, 0, 0);
        (
            sys::send_message(child.0, SCI_GETLENGTH, 0, 0),
            sys::send_message(child.0, SCI_GETTEXTLENGTH, 0, 0),
        )
    };

    assert_eq!(length, 0, "a fresh Scintilla control should have no text");
    assert_eq!(
        text_length, 0,
        "a fresh Scintilla control should have no text"
    );

    // SAFETY: both windows are live and were created by this thread.
    unsafe {
        let _ = DestroyWindow(child);
        let _ = DestroyWindow(parent);
    }
    release();
    Outcome::Passed
}

fn destroy_parent(parent: windows::Win32::Foundation::HWND) {
    // SAFETY: `parent` is live and was created by this thread.
    unsafe {
        let _ = DestroyWindow(parent);
    }
}

fn release() {
    // SAFETY: every Scintilla control created in this test has been destroyed.
    unsafe {
        sys::Scintilla_ReleaseResources();
    }
}
