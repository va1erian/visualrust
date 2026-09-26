//! Dyon code generation: turn a [`Form`] model into runnable Dyon source.
//!
//! [`generate`] is a pure function of the model and its options, so regenerating
//! an unchanged form is byte-identical. It emits exactly one Dyon program: a
//! `main` that builds the window and its controls, plus (optionally) empty
//! handler stubs. Nothing here reads or writes files, and nothing here
//! implements the runtime: [`generate`] targets the native `ui_*` surface that
//! the runtime gap issues provide.
//!
//! # Target API surface
//!
//! The generated program calls the following native functions. They are the
//! contract with [`#136`] (constructors and setters), [`#137`] (events) and
//! [`#138`] (anchored layout) and are not implemented in `vr-forms`:
//!
//! ```text
//! ui_window(title, width, height)              -> Window
//! ui_free(width, height)                       -> Free    // Layout::free root
//! ui_add(root, widget)                         -> ()
//! ui_<kind>(text, x, y, w, h)                  -> Widget  // one per ControlKind
//! ui_set_text(widget, text)
//! ui_set_tooltip(widget, text)
//! ui_set_enabled(widget, flag)
//! ui_set_visible(widget, flag)
//! ui_set_multiline(widget, flag)               // Edit
//! ui_set_password(widget, flag)                // Edit
//! ui_set_placeholder(widget, text)             // Edit
//! ui_set_checked(widget, flag)                 // CheckBox
//! ui_set_items(widget, [text, ...])            // choice/list kinds
//! ui_set_selected(widget, text)                // RadioGroup, ComboBox
//! ui_set_selected_index(widget, index)         // Tabs
//! ui_set_editable(widget, flag)                // ComboBox
//! ui_set_columns(widget, [text, ...])          // ListView, GridView
//! ui_set_border(widget, flag)                  // Panel
//! ui_set_scroll_bars(widget, "none"|"vertical"|"horizontal"|"both")
//! ui_set_range(widget, min, max)               // ProgressBar, Slider
//! ui_set_value(widget, value)                  // ProgressBar, Slider
//! ui_set_orientation(widget, "horizontal"|"vertical") // Slider
//! ui_set_color(widget, "#rrggbb")              // ColorPicker
//! ui_set_wrap(widget, flag)                    // FlowText
//! ui_anchor(widget, "top_left"|...|"fill")
//! ui_on(widget, event, "on_<event>_<Control>")
//! ui_run(window, root)
//! ```
//!
//! Caption widgets ([`ControlKind::Button`], [`ControlKind::Label`],
//! [`ControlKind::CheckBox`], [`ControlKind::GroupBox`],
//! [`ControlKind::RadioGroup`], [`ControlKind::ComboBox`]) receive their `text`
//! through the constructor; every other kind receives `""` there and, when the
//! model carries text, through `ui_set_text` — so the setter path is present in
//! the generated source. Anchors use the same snake_case names as the
//! `.vrform` `anchor` field and [`design::anchor_to_xui`]'s xui mapping.
//!
//! Not every model property has a dedicated call yet: menu separators, toolbar
//! separators and tree nesting are flattened to the item labels they carry
//! (`ui_set_items`). The runtime bindings that preserve those shapes are part
//! of [`#136`].
//!
//! # Code-behind
//!
//! Handler stubs are emitted by [`EmitHandlers::Stubs`]. A later merge pass
//! reads the existing file, keeps every handler body already present, and only
//! appends the missing ones; callers that own the merge should pass
//! [`EmitHandlers::None`] so a regeneration never rewrites a body the user has
//! edited.
//!
//! [`design::anchor_to_xui`]: crate::design::anchor_to_xui
//! [`#136`]: https://github.com/va1erian/visualrust/issues/136
//! [`#137`]: https://github.com/va1erian/visualrust/issues/137
//! [`#138`]: https://github.com/va1erian/visualrust/issues/138

mod emit;
mod escape;

use std::fmt::Write as _;

use emit::Handlers;
use escape::quoted;

use crate::model::Form;

/// The indentation unit [`generate`] uses per nesting level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indent {
    /// A fixed number of spaces per level.
    Spaces(u8),
    /// One tab per level.
    Tab,
}

impl Indent {
    fn unit(self) -> String {
        match self {
            Indent::Spaces(count) => " ".repeat(count as usize),
            Indent::Tab => "\t".to_owned(),
        }
    }
}

/// Whether [`generate`] writes empty handler functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitHandlers {
    /// Write no handler functions; a separate code-behind file must supply
    /// them. Use this when merging into a file whose bodies are authoritative.
    None,
    /// Write an empty `fn on_<event>_<Control>() {}` for every wired event.
    Stubs,
}

/// Tuning for [`generate`]. [`CodegenOptions::default`] is the common case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenOptions {
    /// Name used in the generated header comment. An empty or blank value falls
    /// back to the form's own name.
    pub module: String,
    /// Whether to emit handler stubs.
    pub handlers: EmitHandlers,
    /// The indentation unit.
    pub indent: Indent,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            module: "main".to_owned(),
            handlers: EmitHandlers::Stubs,
            indent: Indent::Spaces(4),
        }
    }
}

/// Generates the Dyon source for `form`.
///
/// Assumes `form` already passed [`Form::validate`]; an invalid model still
/// produces text, but the text may not reflect a usable design. The result
/// depends only on `form` and `options`, so it is safe to regenerate and diff.
pub fn generate(form: &Form, options: CodegenOptions) -> String {
    let unit = options.indent.unit();
    let module = if options.module.trim().is_empty() {
        form.name.as_str()
    } else {
        options.module.as_str()
    };
    let width = emit::number(form.size.width.get());
    let height = emit::number(form.size.height.get());

    let mut out = String::new();
    let _ = writeln!(
        out,
        "// Generated by vr-forms from `{}`. Regenerate instead of editing;",
        module.replace(['\n', '\r'], " ")
    );
    out.push_str("// handler bodies are preserved by the code-behind merge.\n\n");
    out.push_str("fn main() {\n");
    let _ = writeln!(
        out,
        "{unit}w := ui_window({}, {width}, {height})",
        quoted(&form.name)
    );
    let _ = writeln!(out, "{unit}root := ui_free({width}, {height})");

    let mut handlers = Handlers::default();
    for (index, control) in form.controls.iter().enumerate() {
        emit::emit_control(&mut out, control, index, &unit, &mut handlers);
    }

    let _ = writeln!(out, "{unit}ui_run(w, root)");
    out.push_str("}\n");

    if options.handlers == EmitHandlers::Stubs {
        for name in handlers.names() {
            let _ = write!(out, "\nfn {name}() {{\n}}\n");
        }
    }
    out
}
