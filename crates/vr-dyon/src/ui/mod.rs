//! Native `ui_*` bindings: build and run a xui window entirely from Dyon.
//!
//! The Dyon-facing API is the form-generator's target surface:
//!
//! ```dyon
//! fn main() {
//!     w := ui_window("Title", 480, 320)
//!     root := ui_free(480, 320)
//!     button := ui_button("Greet", 10, 10, 90, 40)
//!     ui_add(root, button)
//!     ui_anchor(button, "fill")
//!     ui_on(button, "click", "on_click_Greet")
//!     ui_run(w, root)
//! }
//! ```
//!
//! # No Dyon re-entrancy
//!
//! `ui_run` does **not** block. It registers the window, the widget tree and the
//! event map in a per-thread builder and returns; [`runtime::run`] hands the
//! frozen plan to [`host`] once the program's `main` has returned. The host then
//! starts xui's message loop with an [`App`](xui::App) that owns the Dyon runtime
//! and module, so a handler runs in `App::update` on a program that is no longer
//! on the stack. A handler that calls a setter mutates the same plan the host
//! materialised, and never pumps the queue.
//!
//! [`runtime::run`]: crate::DyonRuntime::run
//!
//! # Degradation
//!
//! A widget family that xui has no direct child control for (the model-driven
//! lists, trees, tabs and menu strip) is materialised as a [`Label`] placeholder
//! rather than failing, so a generated form always opens. The host logs the
//! substitution when `VR_DYON_UI_DEBUG` is set.
//!
//! [`Label`]: xui::Label

use std::cell::Cell;

mod arity;
mod bindings;
mod host;
pub(crate) mod plan;
mod widget;

pub(crate) use arity::set_arities;
pub(crate) use bindings::register;
pub(crate) use host::{HostError, run as run_host};
pub(crate) use plan::take_pending;

use plan::{EventKind, HandleId};

/// The message a widget maps its events to.
///
/// It carries the widget handle and the event kind; the host looks up the named
/// Dyon handler for the pair. Kept a plain `Copy` value so xui can queue it
/// without owning any widget state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UiEvent {
    pub(crate) handle: HandleId,
    pub(crate) event: EventKind,
}

/// Whether a Dyon run asked for and got a window.
///
/// The native functions cannot report this through their `Result` alone (a
/// headless session and a script bug both surface as runtime errors), so the
/// status is kept per-thread and consumed by [`DyonRuntime::run`].
///
/// [`DyonRuntime::run`]: crate::DyonRuntime::run
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiStatus {
    /// No `ui_run` call was made.
    NotRun,
    /// A window was created and the message loop returned cleanly.
    Ran,
    /// No window could be created on this session.
    NoWindow,
}

thread_local! {
    static STATUS: Cell<UiStatus> = const { Cell::new(UiStatus::NotRun) };
}

/// Clears the per-thread status and plan before a program runs.
pub(crate) fn reset_status() {
    STATUS.with(|status| status.set(UiStatus::NotRun));
    plan::reset();
}

/// Records the outcome of a hosted run.
pub(crate) fn set_status(status: UiStatus) {
    STATUS.with(|current| current.set(status));
}

/// Reads and clears the per-thread status after a program runs.
pub(crate) fn take_status() -> UiStatus {
    STATUS.with(|status| status.replace(UiStatus::NotRun))
}
