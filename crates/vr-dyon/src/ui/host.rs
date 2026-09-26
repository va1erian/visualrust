//! The xui [`App`] that owns the Dyon runtime and the live widgets.
//!
//! The host is entered from [`DyonRuntime::run`](crate::DyonRuntime::run) after
//! the program's `main` returned, so the Dyon virtual machine is off the stack
//! when a handler runs. That is the whole reason `ui_run` had to stop blocking:
//! a handler can call a setter, and a setter can touch a widget, but it can
//! never pump the message queue and re-enter Dyon.
//!
//! Every widget the plan asked for is materialised in [`widget`](super::widget);
//! this module owns the widget list, installs the free layout and dispatches
//! events to the named Dyon functions.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use dyon::embed::PushVariable;
use dyon::{Module, Runtime, Variable};
use xui::{App, Layout, Size, Ui, WindowSpec, dip};

use super::UiEvent;
use super::plan::{EventKind, HandleId, PendingUi};
use super::widget::{EventPayload, Live, build_widget};

/// Environment variable that makes the window close itself, in milliseconds.
const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";

/// Why the host could not run.
pub(crate) enum HostError {
    /// The session cannot create a top-level window (headless/CI).
    NoWindow,
    /// A widget or the loop failed after a window existed.
    Failed(String),
}

/// The live widgets, keyed by the handle the program used.
struct LiveStore {
    widgets: Vec<Live>,
    by_handle: HashMap<HandleId, usize>,
}

impl LiveStore {
    fn payload_for(&self, handle: HandleId, event: EventKind) -> EventPayload {
        match self.by_handle.get(&handle) {
            Some(&index) => self.widgets[index].payload(event),
            None => EventPayload::None,
        }
    }

    fn mutate(&self, handle: HandleId, f: impl FnOnce(&Live)) {
        if let Some(&index) = self.by_handle.get(&handle) {
            f(&self.widgets[index]);
        }
    }
}

thread_local! {
    /// The widgets of the currently hosted window, for setters a Dyon handler
    /// calls while the host owns the widget list. `None` before the loop starts
    /// and after it returns, so a setter during `main` only edits the plan.
    static LIVE: RefCell<Option<Rc<RefCell<LiveStore>>>> = const { RefCell::new(None) };
}

/// Applies a live text change from a handler, if a window is hosted.
pub(super) fn set_live_text(handle: HandleId, text: &str) {
    LIVE.with(|slot| {
        if let Some(store) = slot.borrow().as_ref() {
            store
                .borrow()
                .mutate(handle, |widget| widget.set_text(text));
        }
    });
}

/// Applies a live enabled change from a handler, if a window is hosted.
pub(super) fn set_live_enabled(handle: HandleId, enabled: bool) {
    LIVE.with(|slot| {
        if let Some(store) = slot.borrow().as_ref() {
            store
                .borrow()
                .mutate(handle, |widget| widget.set_enabled(enabled));
        }
    });
}

/// Applies a live visibility change from a handler, if a window is hosted.
pub(super) fn set_live_visible(handle: HandleId, visible: bool) {
    LIVE.with(|slot| {
        if let Some(store) = slot.borrow().as_ref() {
            store
                .borrow()
                .mutate(handle, |widget| widget.set_visible(visible));
        }
    });
}

/// Builds the window, materialises the plan and runs the message loop.
pub(crate) fn run(
    pending: PendingUi,
    runtime: Runtime,
    module: Arc<Module>,
) -> Result<(), HostError> {
    let spec = WindowSpec::new(pending.window.title.clone())
        .size(dip(pending.window.width), dip(pending.window.height));
    // A failed widget build cannot be returned through `run_app`'s closure, so
    // the error is parked here and read once the loop returns.
    let failure: Rc<RefCell<Option<HostError>>> = Rc::new(RefCell::new(None));
    let failure_in = Rc::clone(&failure);
    let outcome = xui::run_app(spec, move |ui| {
        build(ui, pending, runtime, module, failure_in)
    });
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error);
    }
    outcome.map_err(classify)
}

/// Materialises the plan, installs the free layout and returns the app that
/// owns the widgets for the window's lifetime.
fn build(
    ui: &mut Ui<UiEvent>,
    pending: PendingUi,
    runtime: Runtime,
    module: Arc<Module>,
    failure: Rc<RefCell<Option<HostError>>>,
) -> HostApp {
    let dpi = ui.dpi();
    let mut widgets = Vec::new();
    let mut by_handle = HashMap::new();
    let mut handlers = HashMap::new();
    // The free layout anchors against the form's design origin, not the window
    // size: `ui_window` sizes the frame and `ui_free` sizes the design surface.
    let origin = Size::new(
        pending.origin.0.round() as i32,
        pending.origin.1.round() as i32,
    );
    let mut layout = Layout::free(origin);

    for plan in &pending.widgets {
        // The free root is a layout origin, not a control.
        if plan.id == pending.root {
            continue;
        }
        match build_widget(ui, plan, dpi) {
            Ok(widget) => {
                layout = layout.item(widget.anchor_item(plan.anchor));
                by_handle.insert(plan.id, widgets.len());
                widgets.push(widget);
                for (event, name) in &plan.handlers {
                    handlers.insert((plan.id, *event), name.clone());
                }
            }
            Err(error) => {
                *failure.borrow_mut() = Some(error);
                ui.quit();
                break;
            }
        }
    }

    ui.set_layout(layout);
    arm_autoclose(ui);

    let store = Rc::new(RefCell::new(LiveStore { widgets, by_handle }));
    LIVE.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&store)));

    // Test dispatches run once the loop starts; emitting here lets `run_app`
    // drain them after `make`, so a handler never runs while Dyon is on the
    // stack.
    for (handle, event) in pending.invocations {
        ui.emit(UiEvent { handle, event });
    }

    HostApp {
        runtime,
        module,
        handlers,
        live: store,
        arities: pending.arities,
        last_error: None,
    }
}

/// The host application: the Dyon runtime plus the widgets it drives.
struct HostApp {
    runtime: Runtime,
    module: Arc<Module>,
    handlers: HashMap<(HandleId, EventKind), String>,
    live: Rc<RefCell<LiveStore>>,
    arities: HashMap<String, usize>,
    /// The most recent handler failure, kept for diagnostics; the loop ignores
    /// it so one bad handler cannot kill the window.
    #[allow(dead_code)]
    last_error: Option<String>,
}

impl App for HostApp {
    type Msg = UiEvent;

    /// Calls the Dyon function bound to the widget's event.
    ///
    /// Exactly one Dyon call runs here and it never pumps the message queue, so
    /// Dyon is never re-entered. A failed call is retained in `last_error` and
    /// does not disturb the loop.
    fn update(&mut self, msg: UiEvent, _ui: &mut Ui<UiEvent>) {
        let Some(function) = self.handlers.get(&(msg.handle, msg.event)).cloned() else {
            return;
        };
        let payload = self.live.borrow().payload_for(msg.handle, msg.event);
        let arity = self.arities.get(&function).copied().unwrap_or(0);
        let args = payload_args(payload, arity);
        if let Err(message) = self.runtime.call_str(&function, &args, &self.module) {
            self.last_error = Some(format!("Dyon handler `{function}` failed: {message}"));
        }
    }
}

impl Drop for HostApp {
    fn drop(&mut self) {
        LIVE.with(|slot| *slot.borrow_mut() = None);
    }
}

/// Turns a payload into the argument list for a handler of the given arity: a
/// zero-argument generated handler gets none, and one that declares a parameter
/// receives the payload.
fn payload_args(payload: EventPayload, arity: usize) -> Vec<Variable> {
    if arity == 0 {
        return Vec::new();
    }
    match payload {
        EventPayload::Text(text) => vec![text.push_var()],
        EventPayload::Checked(checked) => vec![checked.push_var()],
        EventPayload::Value(value) => vec![value.push_var()],
        EventPayload::None => Vec::new(),
    }
}

/// Arms `xui_DEMO_AUTOCLOSE_MS` so a headless run leaves on its own.
fn arm_autoclose(ui: &mut Ui<UiEvent>) {
    let Some(millis) = std::env::var(AUTOCLOSE_ENV)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return;
    };
    let Ok(timer) = ui.set_timer(millis) else {
        return;
    };
    let timer_ui = ui.clone();
    ui.on_timer(move |fired| {
        if fired == timer {
            timer_ui.quit();
        }
        None
    });
}

/// A window- or control-creation failure means the session cannot build the UI
/// (no desktop, or common controls unavailable), which a test skips; every
/// other error is a real failure.
pub(super) fn classify(error: xui::Error) -> HostError {
    match error {
        xui::Error::ClassRegistration { .. }
        | xui::Error::CreateWindow { .. }
        | xui::Error::CreateControl(_)
        | xui::Error::ControlsUnavailable => HostError::NoWindow,
        other => HostError::Failed(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_arity_handlers_get_no_arguments() {
        assert!(payload_args(EventPayload::Text("hi".into()), 0).is_empty());
    }

    #[test]
    fn payload_is_typed_for_a_one_argument_handler() {
        let args = payload_args(EventPayload::Checked(true), 1);
        assert_eq!(args.len(), 1);
    }
}
