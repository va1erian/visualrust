//! Native `ui_*` bindings: build and run a xui window entirely from Dyon.
//!
//! The Dyon-facing API is a small builder vocabulary:
//!
//! ```dyon
//! fn main() {
//!     win := ui_window("Title", 480, 320)
//!     root := ui_column([
//!         ui_label("Hello from Dyon"),
//!         ui_row([ui_label("left"), ui_label("right")])
//!     ])
//!     ui_run(win, root)
//! }
//! ```
//!
//! `ui_window` returns a window plan and `ui_label` / `ui_column` / `ui_row`
//! return widget plans, each stored in the Dyon runtime as a custom object
//! ([`dyon::RustObject`] = `Arc<Mutex<dyn Any>>`) exactly as the issue asks.
//! They are *plans*, not live widgets, because xui only exposes a `Ui` to
//! the closure passed to [`xui::run_app`]; the concrete `Label` children are
//! created there, inside [`ui_run`], and kept alive by the app for the window's
//! lifetime. `ui_run` blocks in the xui message loop until the window closes.
//!
//! The loop ends on its own when a non-zero `xui_DEMO_AUTOCLOSE_MS`
//! environment variable is set, which is how a headless test or example closes
//! the window. [`UiStatus`] records whether a window was ever created so the
//! runtime can turn "this session has no desktop" into [`DyonError::NoWindow`]
//! and let a test skip instead of fail.
//!
//! [`DyonError::NoWindow`]: crate::DyonError::NoWindow

use std::cell::Cell;
use std::sync::Arc;

use dyon::embed::to_rust_object;
use dyon::{Dfn, Module, Runtime, RustObject, Type, Variable};
use xui::Rect;

/// Environment variable that makes the window close itself, in milliseconds.
const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";
/// Initial label bounds; the installed layout moves the label on the first
/// relayout, but the first measure needs a non-zero natural height.
const LABEL_BOUNDS: Rect = Rect::new(0, 0, 160, 24);
/// Gap between items in a generated row/column, in design units.
const STACK_SPACING: f32 = 6.0;

/// A window the Dyon program asked for, resolved by `ui_run`.
#[derive(Clone, Debug)]
struct WindowPlan {
    title: String,
    width: f32,
    height: f32,
}

/// A widget the Dyon program asked for. Only the vocabulary issue #6 names;
/// more controls become variants as later issues add them.
#[derive(Clone, Debug)]
enum WidgetPlan {
    Label(String),
    Column(Vec<WidgetPlan>),
    Row(Vec<WidgetPlan>),
}

/// Whether a Dyon run created a window.
///
/// The native functions cannot report this through their `Result` alone (a
/// headless session and a script bug both surface as runtime errors), so the
/// status is kept per-thread and consumed by [`DyonRuntime::run`].
///
/// [`DyonRuntime::run`]: crate::DyonRuntime::run
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiStatus {
    /// No `ui_run` call was made, or it failed after a window existed.
    NotRun,
    /// A window was created and the message loop returned cleanly.
    Ran,
    /// No window could be created on this session.
    NoWindow,
}

thread_local! {
    static STATUS: Cell<UiStatus> = const { Cell::new(UiStatus::NotRun) };
}

/// Clears the per-thread status before a program runs.
pub(crate) fn reset_status() {
    STATUS.with(|status| status.set(UiStatus::NotRun));
}

/// Reads and clears the per-thread status after a program runs.
pub(crate) fn take_status() -> UiStatus {
    STATUS.with(|status| status.replace(UiStatus::NotRun))
}

/// Registers the `ui_*` functions into `module`.
///
/// Must run before source is loaded so the lifetime checker sees the signatures.
pub(crate) fn register(module: &mut Module) {
    let window_ty = ad_hoc("Window");
    let widget_ty = ad_hoc("Widget");
    module.add_str(
        "ui_window",
        ui_window,
        Dfn::nl(vec![Type::Str, Type::F64, Type::F64], window_ty.clone()),
    );
    module.add_str(
        "ui_label",
        ui_label,
        Dfn::nl(vec![Type::Str], widget_ty.clone()),
    );
    module.add_str(
        "ui_column",
        ui_column,
        Dfn::nl(vec![Type::Array(Box::new(Type::Any))], widget_ty.clone()),
    );
    module.add_str(
        "ui_row",
        ui_row,
        Dfn::nl(vec![Type::Array(Box::new(Type::Any))], widget_ty.clone()),
    );
    module.add_str(
        "ui_run",
        ui_run,
        Dfn::nl(vec![window_ty, widget_ty], Type::Void),
    );
}

/// An ad-hoc type named `name`, so a plan returned by one binding is accepted
/// only by bindings that expect the same kind of plan.
fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}

fn widget_object(plan: WidgetPlan) -> Variable {
    Variable::RustObject(to_rust_object(plan))
}

fn ui_window(rt: &mut Runtime) -> Result<Variable, String> {
    let height: f64 = rt.pop()?;
    let width: f64 = rt.pop()?;
    let title: String = rt.pop()?;
    Ok(Variable::RustObject(to_rust_object(WindowPlan {
        title,
        width: width as f32,
        height: height as f32,
    })))
}

fn ui_label(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(widget_object(WidgetPlan::Label(text)))
}

fn ui_column(rt: &mut Runtime) -> Result<Variable, String> {
    let items: Variable = rt.pop()?;
    Ok(widget_object(WidgetPlan::Column(children(&items)?)))
}

fn ui_row(rt: &mut Runtime) -> Result<Variable, String> {
    let items: Variable = rt.pop()?;
    Ok(widget_object(WidgetPlan::Row(children(&items)?)))
}

fn ui_run(rt: &mut Runtime) -> Result<(), String> {
    let root: RustObject = rt.pop()?;
    let window: RustObject = rt.pop()?;
    let window = plan::<WindowPlan>(&window)?;
    let root = plan::<WidgetPlan>(&root)?;
    match host::run(window, root) {
        Ok(()) => {
            STATUS.with(|status| status.set(UiStatus::Ran));
            Ok(())
        }
        Err(host::HostError::NoWindow) => {
            STATUS.with(|status| status.set(UiStatus::NoWindow));
            Err("could not create an interactive window".to_owned())
        }
        Err(host::HostError::Failed(message)) => Err(message),
    }
}

/// Reads a widget-plan array argument, preserving order.
fn children(variable: &Variable) -> Result<Vec<WidgetPlan>, String> {
    let Variable::Array(items) = variable else {
        return Err(format!(
            "expected an array of widgets, got {}",
            variable.typeof_var()
        ));
    };
    items.iter().map(child).collect()
}

fn child(variable: &Variable) -> Result<WidgetPlan, String> {
    let Variable::RustObject(object) = variable else {
        return Err(format!(
            "expected a widget handle, got {}",
            variable.typeof_var()
        ));
    };
    plan::<WidgetPlan>(object)
}

fn plan<T: Clone + 'static>(object: &RustObject) -> Result<T, String> {
    let guard = object
        .lock()
        .map_err(|_| "a UI handle was poisoned".to_owned())?;
    guard
        .downcast_ref::<T>()
        .cloned()
        .ok_or_else(|| format!("expected a {} plan", std::any::type_name::<T>()))
}

/// Creates the xui window: the only place concrete widgets are built.
mod host {
    use std::cell::RefCell;
    use std::rc::Rc;

    use xui::{App, IntoLayoutItem, Label, Layout, LayoutExt, LayoutItem, Ui, WindowSpec, dip};

    use super::{AUTOCLOSE_ENV, LABEL_BOUNDS, STACK_SPACING, WidgetPlan, WindowPlan};

    /// Why `ui_run` could not run a window.
    pub(super) enum HostError {
        /// The session cannot create a top-level window (headless/CI).
        NoWindow,
        /// The window existed but a widget or the loop failed.
        Failed(String),
    }

    /// Builds the window and blocks in the message loop until it closes.
    pub(super) fn run(window: WindowPlan, root: WidgetPlan) -> Result<(), HostError> {
        let spec = WindowSpec::new(window.title).size(dip(window.width), dip(window.height));
        // A failed widget build cannot be returned through `run_app`'s closure,
        // so the error is parked here and read once the loop returns.
        let failure: Rc<RefCell<Option<HostError>>> = Rc::new(RefCell::new(None));
        let failure_in = Rc::clone(&failure);
        let outcome = xui::run_app(spec, move |ui| build_app(ui, root, failure_in));
        if let Some(error) = failure.borrow_mut().take() {
            return Err(error);
        }
        outcome.map_err(classify)
    }

    /// Builds the widget tree and installs it, then returns the app that owns
    /// the widgets for the window's lifetime.
    fn build_app(
        ui: &mut Ui<()>,
        root: WidgetPlan,
        failure: Rc<RefCell<Option<HostError>>>,
    ) -> UiApp {
        let mut labels = Vec::new();
        match build_layout(ui, &root, &mut labels) {
            Ok(layout) => {
                ui.set_layout(layout);
                arm_autoclose(ui);
            }
            Err(error) => {
                *failure.borrow_mut() = Some(error);
                ui.quit();
            }
        }
        UiApp { _labels: labels }
    }

    /// Recursively materialises a plan into a [`Layout`]. A bare label becomes a
    /// one-item column so any widget can be the root.
    fn build_layout(
        ui: &mut Ui<()>,
        plan: &WidgetPlan,
        labels: &mut Vec<Label>,
    ) -> Result<Layout, HostError> {
        match plan {
            WidgetPlan::Column(items) => build_stack(ui, Layout::column(), items, labels),
            WidgetPlan::Row(items) => build_stack(ui, Layout::row(), items, labels),
            WidgetPlan::Label(text) => {
                let label = make_label(ui, text)?;
                let item = label.layout_item();
                labels.push(label);
                Ok(Layout::column().item(item))
            }
        }
    }

    fn build_stack(
        ui: &mut Ui<()>,
        base: Layout,
        items: &[WidgetPlan],
        labels: &mut Vec<Label>,
    ) -> Result<Layout, HostError> {
        let mut layout = base.spacing(dip(STACK_SPACING));
        for item in items {
            layout = layout.item(build_item(ui, item, labels)?);
        }
        Ok(layout)
    }

    fn build_item(
        ui: &mut Ui<()>,
        plan: &WidgetPlan,
        labels: &mut Vec<Label>,
    ) -> Result<LayoutItem, HostError> {
        match plan {
            WidgetPlan::Label(text) => {
                let label = make_label(ui, text)?;
                let item = label.layout_item();
                labels.push(label);
                Ok(item)
            }
            WidgetPlan::Column(_) | WidgetPlan::Row(_) => {
                Ok(build_layout(ui, plan, labels)?.into_layout_item())
            }
        }
    }

    fn make_label(ui: &mut Ui<()>, text: &str) -> Result<Label, HostError> {
        Label::new(ui, LABEL_BOUNDS, text).map_err(classify)
    }

    /// Arms `xui_DEMO_AUTOCLOSE_MS` so a headless run leaves on its own.
    fn arm_autoclose(ui: &mut Ui<()>) {
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

    /// A window- or control-creation failure means the session cannot build the
    /// UI (no desktop, or common controls unavailable), which a test skips;
    /// every other error is a real failure.
    fn classify(error: xui::Error) -> HostError {
        match error {
            xui::Error::ClassRegistration { .. }
            | xui::Error::CreateWindow { .. }
            | xui::Error::CreateControl(_)
            | xui::Error::ControlsUnavailable => HostError::NoWindow,
            other => HostError::Failed(other.to_string()),
        }
    }

    /// Owns the built labels so their `HWND`s outlive `run_app`.
    struct UiApp {
        _labels: Vec<Label>,
    }

    impl App for UiApp {
        type Msg = ();

        fn update(&mut self, _msg: (), _ui: &mut Ui<()>) {}
    }
}
