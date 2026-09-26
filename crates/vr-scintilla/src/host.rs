//! The xui host widget for a real Scintilla control.
//!
//! Scintilla draws itself, so the xui [`CustomWidget`] path here only owns
//! the parent `HWND`; the control is created as its child and resized to fill
//! it. Notifications are Scintilla's own `SCN_*`: the control sends a
//! `WM_NOTIFY` to its parent, which [`crate::sys`] subclasses, so this widget
//! forwards decoded [`Scn`]s to the application without the xui
//! `CustomWidget::input` path.
//!
//! Dyon highlighting is the container-lexer path from `docs/PLAN.md`:
//! `SCI_SETILEXER(NULL)` makes Scintilla ask for styles with
//! `SCN_STYLENEEDED`; the handler restyles from `SCI_GETENDSTYLED` using the
//! [`Lexer`] from `vr-syntax`, then `SCI_SETSTYLING`s the result. The widget
//! maps the xui [`Theme`] to Scintilla styles with
//! [`crate::theme_map`].

use std::cell::RefCell;
use std::rc::Rc;

use xui::gdi::Canvas;
use xui::{AsControl, Control, ControlExt, Custom, CustomWidget, Rect, Size, Theme, Ui};

use vr_syntax::{Lexer, StyleKind};

use crate::handle::Scintilla;
use crate::sys::{NotificationSink, ParentSubclass};
use crate::types::{MarginType, STYLE_DEFAULT, STYLE_LINENUMBER};
use crate::{Error, Result, Scn, theme_map};

/// The xui widget behind the hosted control.
///
/// It paints nothing: Scintilla is a child of this widget's window and covers
/// it completely. `Event` names [`Scn`] for the widget vocabulary even though
/// the notifications arrive through the parent subclass, not [`CustomWidget::input`].
struct ScintillaWidget;

impl CustomWidget for ScintillaWidget {
    type Event = Scn;

    fn paint(&self, _canvas: &Canvas, _bounds: Rect, _theme: &Theme) {
        // Scintilla paints the whole client area as a child window; painting
        // here would only be hidden and would flash a background first.
    }

    fn preferred_size(&self, _dpi: u32) -> Option<Size> {
        // A non-zero initial size so the control has a pane before the first
        // layout; the installed layout overrides it.
        Some(Size::new(480, 320))
    }
}

/// A live Scintilla control hosted in a xui window.
///
/// Owns the xui `Custom` parent, the control parented to it, and the
/// notification subclass; dropping it removes the subclass, destroys the
/// control and then the parent, and leaks nothing.
pub struct ScintillaHost<M: 'static> {
    /// Declared first so the subclass (and the `Rc<Scintilla>` its sink holds)
    /// drops before the control and the parent, keeping Win32 teardown ordered.
    /// Held only for its `Drop`; the notifications flow through it, not through
    /// this field.
    _subclass: ParentSubclass,
    editor: Rc<Scintilla>,
    custom: Custom<ScintillaWidget, M>,
}

impl<M: 'static> ScintillaHost<M> {
    /// Creates the host inside `ui`, with `on_event` mapping each [`Scn`] to
    /// the application's `Msg` (or `None` to ignore it).
    ///
    /// Returns [`Error::CreateControl`] when the xui parent or the control
    /// cannot be created — expected on a session without a desktop — and
    /// [`Error::SubclassParent`] if the parent cannot be subclassed.
    pub fn new(
        ui: &mut Ui<M>,
        on_event: impl Fn(Scn) -> Option<M> + 'static,
    ) -> Result<ScintillaHost<M>> {
        let custom = Custom::new(ui, ScintillaWidget).map_err(|_| Error::CreateControl)?;
        let parent = custom.control().hwnd().raw() as *mut core::ffi::c_void;
        let editor = Rc::new(Scintilla::with_parent(parent)?);

        configure(&editor);
        theme_map::apply(&editor, &ui.theme());
        let bounds = custom.bounds();
        editor.set_bounds(bounds.left, bounds.top, bounds.width(), bounds.height());

        let notifier = Notifier {
            editor: Rc::clone(&editor),
            lexer: RefCell::new(Lexer::new()),
            mapper: Rc::new(on_event),
            ui: ui.clone(),
        };
        let subclass = ParentSubclass::install(editor.raw_hwnd(), Box::new(notifier))
            .ok_or(Error::SubclassParent)?;

        // A `Weak` here (not a second `Rc`) so the control is dropped when
        // `ScintillaHost::editor` drops, before the parent `Custom` does.
        let weak = Rc::downgrade(&editor);
        custom.on_resize(move |rect| {
            if let Some(editor) = weak.upgrade() {
                editor.set_bounds(rect.left, rect.top, rect.width(), rect.height());
            }
        });

        Ok(ScintillaHost {
            _subclass: subclass,
            editor,
            custom,
        })
    }

    /// Replaces the document text, `SCI_SETTEXT`. Scintilla then asks for
    /// styles with `SCN_STYLENEEDED`.
    pub fn set_text(&self, text: &str) {
        self.editor.set_text(text);
    }

    /// Returns the document text, `SCI_GETTEXT`.
    pub fn text(&self) -> Result<String> {
        self.editor.text()
    }

    /// Scrolls to `line`, `SCI_GOTOLINE`.
    pub fn goto_line(&self, line: i32) {
        self.editor.goto_line(line);
    }

    /// Re-applies `theme`'s palette to the control. Call this after
    /// [`Ui::set_theme`]: the xui themed-children pass reaches the `Custom`
    /// parent but cannot know about the foreign control inside it.
    pub fn apply_theme(&self, theme: &Theme) {
        theme_map::apply(&self.editor, theme);
    }
}

impl<M: 'static> AsControl for ScintillaHost<M> {
    fn control(&self) -> &Control {
        self.custom.control()
    }
}

/// The safe sink the parent subclass calls with each decoded notification.
///
/// It restyles on demand and maps the event to the app's `Msg` through
/// [`Ui::emit`], the public equivalent of `WidgetCx::emit`: the sink runs
/// outside a `CustomWidget::input` call, where no `WidgetCx` exists.
struct Notifier<M: 'static> {
    editor: Rc<Scintilla>,
    lexer: RefCell<Lexer>,
    mapper: Rc<dyn Fn(Scn) -> Option<M>>,
    ui: Ui<M>,
}

impl<M: 'static> NotificationSink for Notifier<M> {
    fn notification(&self, scn: Scn) {
        if let Scn::StyleNeeded { position } = &scn {
            restyle(&self.editor, &mut self.lexer.borrow_mut(), *position);
        }
        if let Some(msg) = (self.mapper)(scn) {
            self.ui.emit(msg);
        }
    }
}

/// The editor's fixed-width face. Columns and indentation only line up in a
/// monospace font; Scintilla keeps its default if the face is not installed.
const EDITOR_FONT: &str = "Consolas";

/// One-time control configuration: the container lexer, a line-number margin,
/// tabs and a point size for every Dyon style.
fn configure(editor: &Scintilla) {
    editor.set_container_lexer();
    editor.set_margin_type(0, MarginType::Number);
    editor.set_margin_width(0, 48);
    for style in StyleKind::ALL {
        editor.set_style_font(style.index(), EDITOR_FONT);
        editor.set_style_size(style.index(), 11);
    }
    editor.set_style_font(STYLE_DEFAULT, EDITOR_FONT);
    editor.set_style_size(STYLE_DEFAULT, 11);
    editor.set_style_font(STYLE_LINENUMBER, EDITOR_FONT);
    editor.set_style_size(STYLE_LINENUMBER, 10);
}

/// Restyles the region Scintilla asked for: from the start of the line
/// containing `SCI_GETENDSTYLED` up to `position`, using the incremental Dyon
/// [`Lexer`].
///
/// Works in byte positions, matching both Scintilla's UTF-8 positions and the
/// lexer's byte-offset [`StyleRun`](vr_syntax::StyleRun)s.
fn restyle(editor: &Scintilla, lexer: &mut Lexer, position: isize) {
    let Ok(position) = usize::try_from(position) else {
        return;
    };
    let styled = editor.end_styled();
    let line = editor.line_from_position(i32::try_from(styled).unwrap_or(i32::MAX));
    let Ok(start) = usize::try_from(editor.position_from_line(line)) else {
        return;
    };
    let Ok(text) = editor.text() else {
        return;
    };
    let position = position.min(text.len());
    if position <= start {
        return;
    }

    let runs = lexer.restyle_range(&text, start, position);
    let mut styles = vec![StyleKind::Default.index(); position - start];
    for run in runs {
        let run_start = run.start.saturating_sub(start);
        let run_end = (run_start + run.len).min(styles.len());
        if run_start < run_end {
            styles[run_start..run_end].fill(run.kind.index());
        }
    }
    editor.apply_styles(start, &styles);
}
