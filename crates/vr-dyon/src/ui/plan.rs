//! The plan model a Dyon program builds before [`ui_run`](super::bindings).
//!
//! `ui_run` no longer blocks: it registers the window, the widget tree and the
//! event map in a per-thread [`Builder`] and returns. [`take_pending`] hands the
//! frozen [`PendingUi`] to the host loop once the program's `main` has returned,
//! which is what keeps Dyon out of the message loop's re-entrancy window.
//!
//! Handles are opaque ids wrapped in a Dyon custom object. The plan values
//! themselves live in the builder, so a handle is a stable key rather than a
//! shared pointer: setters can mutate one widget without locking a plan.

use std::cell::RefCell;
use std::collections::HashMap;

use dyon::embed::to_rust_object;
use dyon::{Runtime, RustObject, Variable};
use xui::{Anchor, Rect};

/// A widget or window handle id, unique within one program run.
pub(crate) type HandleId = u32;

/// The anchor maths, delegated to xui so the runtime and the designer cannot
/// drift. Exercised from the anchor test below; the host installs the same
/// engine through [`Layout::free`](xui::Layout::free).
#[cfg(test)]
pub(crate) fn anchored_bounds(
    origin: xui::Size,
    resized: xui::Size,
    bounds: Rect,
    anchor: Anchor,
) -> Rect {
    xui::xui_core::anchored(origin, resized, bounds, anchor)
}

/// A window the program asked for.
#[derive(Clone, Debug)]
pub(crate) struct WindowPlan {
    pub(crate) title: String,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// The Dyon-facing event names, matching the string a generated `ui_on` passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum EventKind {
    Click,
    Change,
    Select,
    Scroll,
}

impl EventKind {
    /// Parses the event name from `ui_on`, rejecting an unknown one so a typo
    /// surfaces at registration rather than silently never firing.
    pub(crate) fn parse(name: &str) -> Result<EventKind, String> {
        match name {
            "click" => Ok(EventKind::Click),
            "change" => Ok(EventKind::Change),
            "select" => Ok(EventKind::Select),
            "scroll" => Ok(EventKind::Scroll),
            other => Err(format!("unknown UI event `{other}`")),
        }
    }
}

/// The widget families the generator emits, in the same order as
/// [`vr_forms::ControlKind`](https://github.com/va1erian/visualrust/issues/136).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WidgetKind {
    Button,
    Edit,
    Label,
    CheckBox,
    RadioGroup,
    ComboBox,
    ListView,
    TreeView,
    GroupBox,
    Panel,
    ScrollView,
    Tabs,
    Menu,
    StatusBar,
    Toolbar,
    ProgressBar,
    Slider,
    GridView,
    ColorPicker,
    FlowText,
}

/// The kind-specific properties a setter writes, applied at materialisation so
/// a handler that calls a setter after the window opened sees it immediately.
///
/// The bag is deliberately the full vocabulary the generator emits; a field a
/// given xui control has no setter for is recorded (so the plan is complete)
/// even though [`host`](super::host) cannot apply it yet.
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Setters {
    pub(crate) multiline: Option<bool>,
    pub(crate) password: Option<bool>,
    pub(crate) placeholder: Option<String>,
    pub(crate) checked: Option<bool>,
    pub(crate) items: Option<Vec<String>>,
    pub(crate) selected: Option<String>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) editable: Option<bool>,
    pub(crate) columns: Option<Vec<String>>,
    pub(crate) border: Option<bool>,
    pub(crate) scroll_bars: Option<String>,
    pub(crate) range: Option<(f64, f64)>,
    pub(crate) value: Option<f64>,
    pub(crate) orientation: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) wrap: Option<bool>,
}

/// One widget the program built, in construction order.
#[derive(Clone, Debug)]
pub(crate) struct WidgetPlan {
    pub(crate) id: HandleId,
    pub(crate) kind: WidgetKind,
    pub(crate) text: String,
    /// Absolute design bounds `(x, y, w, h)` in design units, as Dyon wrote them.
    pub(crate) bounds: Rect,
    pub(crate) anchor: Anchor,
    pub(crate) enabled: bool,
    pub(crate) visible: bool,
    pub(crate) tooltip: Option<String>,
    pub(crate) setters: Setters,
    pub(crate) handlers: Vec<(EventKind, String)>,
}

/// A frozen program UI handed from `ui_run` to the host loop.
#[derive(Clone, Debug)]
pub(crate) struct PendingUi {
    pub(crate) window: WindowPlan,
    pub(crate) root: HandleId,
    /// The `ui_free` design origin, in design units: what the controls were
    /// laid out against before any resize.
    pub(crate) origin: (f32, f32),
    pub(crate) widgets: Vec<WidgetPlan>,
    /// Widget/event pairs to dispatch once the loop starts, for tests that
    /// cannot inject input. Empty for a normal program.
    pub(crate) invocations: Vec<(HandleId, EventKind)>,
    /// Handler name to declared parameter count, derived from the source.
    pub(crate) arities: HashMap<String, usize>,
}

/// Builds plans as the program runs and freezes them for the host.
#[derive(Default)]
pub(crate) struct Builder {
    next_id: HandleId,
    window: Option<WindowPlan>,
    root: Option<HandleId>,
    origin: Option<(f32, f32)>,
    widgets: Vec<WidgetPlan>,
    invocations: Vec<(HandleId, EventKind)>,
    ran: bool,
}

impl Builder {
    fn alloc(&mut self) -> HandleId {
        // Ids start at 1 so a zero handle can mean "none" if one ever needs to.
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.next_id
    }

    fn widget_mut(&mut self, id: HandleId) -> Result<&mut WidgetPlan, String> {
        self.widgets
            .iter_mut()
            .find(|widget| widget.id == id)
            .ok_or_else(|| "a UI handle does not name a widget".to_owned())
    }
}

thread_local! {
    static BUILDER: RefCell<Builder> = RefCell::new(Builder::default());
}

/// An opaque handle a native returns to Dyon; the plan lives in the builder.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Handle {
    pub(crate) id: HandleId,
}

/// Wraps `id` as the Dyon custom object a binding returns.
pub(crate) fn handle_variable(id: HandleId) -> Variable {
    Variable::RustObject(to_rust_object(Handle { id }))
}

/// Reads a handle argument, or a typed error when the variable is not one.
pub(crate) fn pop_handle(rt: &mut Runtime) -> Result<HandleId, String> {
    let object: RustObject = rt.pop()?;
    let guard = object
        .lock()
        .map_err(|_| "a UI handle was poisoned".to_owned())?;
    guard
        .downcast_ref::<Handle>()
        .map(|handle| handle.id)
        .ok_or_else(|| "expected a UI handle".to_owned())
}

/// Clears the per-thread builder before a program runs.
pub(crate) fn reset() {
    BUILDER.with(|builder| *builder.borrow_mut() = Builder::default());
}

/// Runs `f` against the current builder.
fn with_builder<T>(f: impl FnOnce(&mut Builder) -> Result<T, String>) -> Result<T, String> {
    BUILDER.with(|builder| f(&mut builder.borrow_mut()))
}

/// Builds a window plan.
pub(crate) fn window(title: String, width: f32, height: f32) -> Result<HandleId, String> {
    with_builder(|builder| {
        let id = builder.alloc();
        builder.window = Some(WindowPlan {
            title,
            width,
            height,
        });
        Ok(id)
    })
}

/// Builds a free (anchored) root plan.
pub(crate) fn free(width: f32, height: f32) -> Result<HandleId, String> {
    with_builder(|builder| {
        if builder.root.is_some() {
            return Err("ui_free may be called only once".to_owned());
        }
        let id = builder.alloc();
        builder.root = Some(id);
        builder.origin = Some((width, height));
        // The free root carries the design origin as a placeholder widget so the
        // host has one record to read; it is never materialised.
        builder.widgets.push(WidgetPlan {
            id,
            kind: WidgetKind::Panel,
            text: String::new(),
            bounds: Rect::new(0, 0, width as i32, height as i32),
            anchor: Anchor::Fill,
            enabled: true,
            visible: true,
            tooltip: None,
            setters: Setters::default(),
            handlers: Vec::new(),
        });
        Ok(id)
    })
}

/// Adds `child` to `root`. The API only builds a flat tree, so the parent is
/// recorded implicitly by leaving every widget in construction order.
pub(crate) fn add(root: HandleId, child: HandleId) -> Result<(), String> {
    with_builder(|builder| {
        if builder.root != Some(root) {
            return Err("ui_add expects the root returned by ui_free".to_owned());
        }
        if !builder.widgets.iter().any(|widget| widget.id == child) {
            return Err("ui_add expects a widget built by a ui_* constructor".to_owned());
        }
        Ok(())
    })
}

/// Inserts a widget and returns its handle.
pub(crate) fn widget(
    kind: WidgetKind,
    text: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<HandleId, String> {
    with_builder(|builder| {
        let id = builder.alloc();
        builder.widgets.push(WidgetPlan {
            id,
            kind,
            text,
            bounds: Rect::new(x as i32, y as i32, (x + width) as i32, (y + height) as i32),
            anchor: Anchor::TopLeft,
            enabled: true,
            visible: true,
            tooltip: None,
            setters: Setters::default(),
            handlers: Vec::new(),
        });
        Ok(id)
    })
}

/// Writes to one field of the addressed widget's setters.
pub(crate) fn setters(
    id: HandleId,
    f: impl FnOnce(&mut WidgetPlan) -> Result<(), String>,
) -> Result<(), String> {
    with_builder(|builder| {
        let widget = builder.widget_mut(id)?;
        f(widget)
    })
}

/// Parses and stores an anchor name, mapping the snake_case `.vrform` names.
pub(crate) fn set_anchor(id: HandleId, name: &str) -> Result<(), String> {
    let anchor = anchor_name(name)?;
    setters(id, |widget| {
        widget.anchor = anchor;
        Ok(())
    })
}

/// Registers a named Dyon handler for one of the widget's events.
pub(crate) fn on(id: HandleId, event: &str, handler: &str) -> Result<(), String> {
    let event = EventKind::parse(event)?;
    setters(id, |widget| {
        widget.handlers.push((event, handler.to_owned()));
        Ok(())
    })
}

/// Queues a dispatch for the host loop to run once, for tests without input.
pub(crate) fn invoke(id: HandleId, event: &str) -> Result<(), String> {
    let event = EventKind::parse(event)?;
    with_builder(|builder| {
        if !builder.widgets.iter().any(|widget| widget.id == id) {
            return Err("ui_invoke expects a widget handle".to_owned());
        }
        builder.invocations.push((id, event));
        Ok(())
    })
}

/// Validates the `ui_run` arguments and marks the UI ready to host.
///
/// `ui_run` returns immediately once this succeeds: the host loop only starts
/// after the program's `main` has returned, so a handler can never re-enter Dyon.
pub(crate) fn mark_ran(window: HandleId, root: HandleId) -> Result<(), String> {
    with_builder(|builder| {
        builder
            .window
            .as_ref()
            .ok_or_else(|| "ui_run expects a window returned by ui_window".to_owned())?;
        if builder.root != Some(root) {
            return Err("ui_run expects the root returned by ui_free".to_owned());
        }
        let _ = window;
        builder.ran = true;
        Ok(())
    })
}

/// Freezes the builder into a [`PendingUi`], or `None` when no window was made
/// or `ui_run` was never called.
pub(crate) fn take_pending() -> Option<PendingUi> {
    let arities = super::arity::arities();
    BUILDER.with(|builder| {
        let mut builder = builder.borrow_mut();
        if !builder.ran {
            return None;
        }
        let window = builder.window.take()?;
        let root = builder.root.take()?;
        let origin = builder.origin.take()?;
        let widgets = std::mem::take(&mut builder.widgets);
        let invocations = std::mem::take(&mut builder.invocations);
        *builder = Builder::default();
        Some(PendingUi {
            window,
            root,
            origin,
            widgets,
            invocations,
            arities,
        })
    })
}

/// The snake_case anchor names accepted by `ui_anchor`.
pub(crate) fn anchor_name(name: &str) -> Result<Anchor, String> {
    Ok(match name {
        "top_left" => Anchor::TopLeft,
        "top" => Anchor::Top,
        "top_right" => Anchor::TopRight,
        "left" => Anchor::Left,
        "center" => Anchor::Center,
        "right" => Anchor::Right,
        "bottom_left" => Anchor::BottomLeft,
        "bottom" => Anchor::Bottom,
        "bottom_right" => Anchor::BottomRight,
        "stretch_horizontal" => Anchor::StretchHorizontal,
        "stretch_vertical" => Anchor::StretchVertical,
        "fill" => Anchor::Fill,
        other => return Err(format!("unknown anchor `{other}`")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xui::Size;

    #[test]
    fn anchor_names_map_to_xui_anchors() {
        assert_eq!(anchor_name("fill"), Ok(Anchor::Fill));
        assert_eq!(anchor_name("top_left"), Ok(Anchor::TopLeft));
        assert_eq!(
            anchor_name("stretch_horizontal"),
            Ok(Anchor::StretchHorizontal)
        );
        assert!(anchor_name("sideways").is_err());
    }

    #[test]
    fn a_fill_widget_grows_with_the_parent() {
        let origin = Size::new(400, 300);
        let resized = Size::new(600, 500);
        let bounds = Rect::new(10, 10, 110, 60);
        let grown = anchored_bounds(origin, resized, bounds, Anchor::Fill);
        assert!(grown.width() > bounds.width(), "fill width grew: {grown:?}");
        assert!(
            grown.height() > bounds.height(),
            "fill height grew: {grown:?}"
        );
    }

    #[test]
    fn take_pending_is_none_without_a_window() {
        reset();
        assert!(take_pending().is_none());
    }
}
