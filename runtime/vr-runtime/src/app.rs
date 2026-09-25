//! The [`win32ui::App`] host that dispatches UI events to Dyon handlers.

use std::collections::HashMap;

use thiserror::Error;
use vr_dyon::{DyonError, DyonRuntime, Variable};
use win32ui::{App, Ui};

/// A UI event slot a Dyon program can bind a handler function to.
///
/// The runtime holds no opinion about the function body; it only maps the slot
/// to a name and calls it. More slots are added as widget events stabilise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandlerId {
    Click,
    Change,
    Timer,
    Close,
}

/// The outcome of dispatching one UI event to the Dyon runtime.
#[derive(Debug, Error)]
pub enum DispatchError {
    /// No Dyon function is bound to the event.
    #[error("no Dyon handler registered for `{event:?}`")]
    Unmapped { event: HandlerId },
    /// The bound Dyon function failed.
    #[error(transparent)]
    Call(#[from] DyonError),
}

/// Something that can call a named Dyon function with arguments.
///
/// Hiding the call behind a trait keeps [`dispatch`] testable without a window:
/// a test can hand in a real [`dyon::Runtime`](dyon::Runtime) carrying its own
/// native functions, while [`DyonRuntime`] is the production implementation.
trait HandlerInvoker {
    fn invoke(&mut self, function: &str, args: &[Variable]) -> Result<(), DyonError>;
}

impl HandlerInvoker for DyonRuntime {
    fn invoke(&mut self, function: &str, args: &[Variable]) -> Result<(), DyonError> {
        self.call(function, args)
    }
}

/// The event to Dyon function-name map.
#[derive(Debug, Default)]
struct HandlerMap {
    functions: HashMap<HandlerId, String>,
}

impl HandlerMap {
    fn bind(&mut self, event: HandlerId, function: impl Into<String>) {
        self.functions.insert(event, function.into());
    }

    fn function(&self, event: HandlerId) -> Option<&str> {
        self.functions.get(&event).map(String::as_str)
    }
}

/// Calls the Dyon function bound to `event`.
///
/// This is the whole dispatch step, kept free of win32ui types so it runs
/// headless. It makes exactly one Dyon call and never pumps the message queue,
/// so a caller inside `App::update` cannot re-enter Dyon through it.
fn dispatch(
    handlers: &HandlerMap,
    invoker: &mut impl HandlerInvoker,
    event: HandlerId,
) -> Result<(), DispatchError> {
    let function = handlers
        .function(event)
        .ok_or(DispatchError::Unmapped { event })?;
    invoker.invoke(function, &[]).map_err(DispatchError::from)
}

/// A packaged application: a compiled Dyon program hosted in a win32ui window.
pub struct RuntimeApp {
    runtime: DyonRuntime,
    handlers: HandlerMap,
    last_error: Option<DispatchError>,
}

impl RuntimeApp {
    /// Compiles `source` and builds an app with no handlers bound yet.
    ///
    /// A parse or lifetime error in `source` is returned as
    /// [`DyonError::Compile`] instead of reaching the window.
    pub fn from_source(source_name: &str, source: &str) -> Result<Self, DyonError> {
        Ok(Self {
            runtime: DyonRuntime::from_source(source_name, source)?,
            handlers: HandlerMap::default(),
            last_error: None,
        })
    }

    /// Binds `event` to the Dyon function named `function`.
    pub fn bind(&mut self, event: HandlerId, function: impl Into<String>) {
        self.handlers.bind(event, function);
    }

    /// The error from the most recent [`App::update`], if that update failed.
    pub fn last_error(&self) -> Option<&DispatchError> {
        self.last_error.as_ref()
    }

    /// Runs the program's `main` function, which conventionally builds widgets.
    pub fn run_main(&mut self) -> Result<(), DyonError> {
        self.runtime.run()
    }
}

impl App for RuntimeApp {
    type Msg = HandlerId;

    /// Dispatches one UI event to its bound Dyon function.
    ///
    /// Only one Dyon call runs here and it never pumps the queue, so Dyon is
    /// never re-entered from inside a dispatch. A failed call is retained in
    /// [`last_error`](Self::last_error) and does not disturb the loop.
    fn update(&mut self, msg: Self::Msg, _ui: &mut Ui<Self::Msg>) {
        self.last_error = dispatch(&self.handlers, &mut self.runtime, msg).err();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dyon::{Dfn, Module, Type, dyon_fn, dyon_fn_pop, dyon_macro_items};

    use super::*;

    static RECORDED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

    dyon_fn! {fn record(value: f64) {
        let mut calls = RECORDED.lock().expect("recording mutex is not poisoned");
        calls.push(value as i32);
    }}

    struct RecordingRuntime {
        runtime: dyon::Runtime,
        module: Arc<Module>,
    }

    impl RecordingRuntime {
        fn compile(source: &str) -> Self {
            let mut module = Module::new();
            module.add_str("record", record, Dfn::nl(vec![Type::F64], Type::Void));
            dyon::load_str("handlers.dyon", Arc::new(source.to_owned()), &mut module)
                .expect("test source compiles");
            Self {
                runtime: dyon::Runtime::new(),
                module: Arc::new(module),
            }
        }
    }

    impl HandlerInvoker for RecordingRuntime {
        fn invoke(&mut self, function: &str, args: &[Variable]) -> Result<(), DyonError> {
            self.runtime
                .call_str(function, args, &self.module)
                .map_err(|message| DyonError::Runtime {
                    message,
                    position: None,
                })
        }
    }

    fn recorded() -> Vec<i32> {
        RECORDED
            .lock()
            .expect("recording mutex is not poisoned")
            .clone()
    }

    #[test]
    fn dispatch_calls_the_bound_dyon_functions_in_order() {
        RECORDED
            .lock()
            .expect("recording mutex is not poisoned")
            .clear();
        let source = r#"
            fn on_click() { record(1) }
            fn on_change() { record(2) }
            fn main() {}
        "#;
        let mut invoker = RecordingRuntime::compile(source);
        let mut handlers = HandlerMap::default();
        handlers.bind(HandlerId::Click, "on_click");
        handlers.bind(HandlerId::Change, "on_change");

        dispatch(&handlers, &mut invoker, HandlerId::Click).expect("click is bound");
        dispatch(&handlers, &mut invoker, HandlerId::Change).expect("change is bound");

        assert_eq!(recorded(), vec![1, 2]);
    }

    #[test]
    fn dispatch_of_an_unmapped_event_is_a_typed_error() {
        let mut invoker = RecordingRuntime::compile("fn main() {}\n");
        let handlers = HandlerMap::default();

        let error =
            dispatch(&handlers, &mut invoker, HandlerId::Timer).expect_err("timer is unmapped");

        assert!(matches!(
            error,
            DispatchError::Unmapped {
                event: HandlerId::Timer
            }
        ));
    }

    #[test]
    fn compile_error_surfaces_as_a_typed_error() {
        let loaded = RuntimeApp::from_source("broken.dyon", "fn main() {\n    x := )\n}\n");
        let error = loaded.err().expect("compile must fail");

        assert!(matches!(error, DyonError::Compile { .. }), "got {error:?}");
    }
}
