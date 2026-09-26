//! The safe [`Scintilla`] handle.
//!
//! It owns a control (and the hidden host window that parents it) and exposes
//! typed methods instead of raw `SCI_*` messages. Decoding and pointer handling
//! live in [`crate::sys`] and [`crate::codec`].

use core::ffi::c_void;

use crate::codec::{decode_c_string, encode_c_string, style_runs};
use crate::sys::Control;
use crate::types::{AnnotationVisible, Color, IndicatorStyle, MarginType, MarkerSymbol};
use crate::{Error, Result};

/// A live Scintilla control.
///
/// Dropping the handle destroys the control and its host window. The handle is
/// bound to the thread that created it, because Win32 windows are.
pub struct Scintilla {
    control: Control,
}

impl Scintilla {
    /// Creates a control on a hidden host window.
    ///
    /// Returns [`Error::CreateControl`] when the session cannot create windows;
    /// tests treat that as a skip rather than a failure.
    pub fn new() -> Result<Scintilla> {
        Control::create()
            .map(|control| Scintilla { control })
            .ok_or(Error::CreateControl)
    }

    /// Creates a visible control parented to `parent` (a xui widget's
    /// child `HWND`).
    ///
    /// The caller owns `parent`; this handle only owns the Scintilla child. The
    /// parent must outlive the returned handle, which the host widget
    /// guarantees by field order.
    pub(crate) fn with_parent(parent: *mut c_void) -> Result<Scintilla> {
        Control::create_parented(parent)
            .map(|control| Scintilla { control })
            .ok_or(Error::CreateControl)
    }

    /// The raw control handle, for the subclass installer. Never leaves the
    /// crate.
    pub(crate) fn raw_hwnd(&self) -> *mut c_void {
        self.control.raw_hwnd()
    }

    /// Moves and resizes the control to `(x, y, width, height)` device pixels,
    /// so it fills its parent's client area.
    pub fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32) {
        self.control.resize(x, y, width, height);
    }

    /// The document length in bytes, `SCI_GETLENGTH`.
    pub fn length(&self) -> usize {
        self.control.length()
    }

    /// Returns the document text, `SCI_GETTEXT`.
    pub fn text(&self) -> Result<String> {
        let mut buffer = vec![0u8; self.control.length() + 1];
        self.control.get_text(&mut buffer);
        decode_c_string(&buffer)
    }

    /// Replaces the document text with `text`, `SCI_SETTEXT`.
    pub fn set_text(&self, text: &str) {
        self.control.set_text(&encode_c_string(text));
    }

    // Styles.

    /// Sets the foreground colour of `style`, `SCI_STYLESETFORE`.
    pub fn set_style_fore(&self, style: u8, color: Color) {
        self.control.style_fore(style as i32, color.to_scintilla());
    }

    /// Sets the background colour of `style`, `SCI_STYLESETBACK`.
    pub fn set_style_back(&self, style: u8, color: Color) {
        self.control.style_back(style as i32, color.to_scintilla());
    }

    /// Sets whether `style` is bold, `SCI_STYLESETBOLD`.
    pub fn set_style_bold(&self, style: u8, bold: bool) {
        self.control.style_bold(style as i32, bold);
    }

    /// Sets whether `style` is italic, `SCI_STYLESETITALIC`.
    pub fn set_style_italic(&self, style: u8, italic: bool) {
        self.control.style_italic(style as i32, italic);
    }

    /// Sets the point size of `style`, `SCI_STYLESETSIZE`.
    pub fn set_style_size(&self, style: u8, points: i32) {
        self.control.style_size(style as i32, points);
    }

    /// Sets the face of `style`, `SCI_STYLESETFONT`. An unknown face is ignored
    /// by Scintilla, which keeps its default.
    pub fn set_style_font(&self, style: u8, name: &str) {
        self.control
            .style_font(style as i32, &encode_c_string(name));
    }

    // Margins and markers.

    /// Sets the content `margin` shows, `SCI_SETMARGINTYPEN`.
    pub fn set_margin_type(&self, margin: i32, kind: MarginType) {
        self.control.set_margin_type(margin, kind as i32);
    }

    /// Sets the pixel width of `margin`, `SCI_SETMARGINWIDTHN`. A width of `0`
    /// hides it.
    pub fn set_margin_width(&self, margin: i32, width: i32) {
        self.control.set_margin_width(margin, width);
    }

    /// Sets the text shown in a text margin on `line`, `SCI_MARGINSETTEXT`.
    pub fn set_margin_text(&self, line: i32, text: &str) {
        self.control.set_margin_text(line, &encode_c_string(text));
    }

    /// Defines the symbol drawn for `marker`, `SCI_MARKERDEFINE`.
    pub fn define_marker(&self, marker: i32, symbol: MarkerSymbol) {
        self.control.define_marker(marker, symbol as i32);
    }

    /// Adds `marker` to `line`, `SCI_MARKERADD`. Returns the marker handle, `-1`
    /// when the line is invalid.
    pub fn add_marker(&self, line: i32, marker: i32) -> isize {
        self.control.add_marker(line, marker)
    }

    /// Removes `marker` from `line`, `SCI_MARKERDELETE`.
    pub fn delete_marker(&self, line: i32, marker: i32) {
        self.control.delete_marker(line, marker);
    }

    // Indicators.

    /// Sets the visual style of `indicator`, `SCI_INDICSETSTYLE`.
    pub fn set_indicator_style(&self, indicator: i32, style: IndicatorStyle) {
        self.control.indicator_set_style(indicator, style as i32);
    }

    /// Sets the colour of `indicator`, `SCI_INDICSETFORE`.
    pub fn set_indicator_fore(&self, indicator: i32, color: Color) {
        self.control
            .indicator_set_fore(indicator, color.to_scintilla());
    }

    /// Sets the alpha of `indicator` (0 transparent, 255 opaque, 256 no alpha),
    /// `SCI_INDICSETALPHA`.
    pub fn set_indicator_alpha(&self, indicator: i32, alpha: i32) {
        self.control.indicator_set_alpha(indicator, alpha);
    }

    /// Selects the indicator that following fill/clear calls act on,
    /// `SCI_SETINDICATORCURRENT`.
    pub fn set_indicator_current(&self, indicator: i32) {
        self.control.set_indicator_current(indicator);
    }

    /// Sets the value carried by the current indicator, `SCI_SETINDICATORVALUE`.
    pub fn set_indicator_value(&self, value: i32) {
        self.control.set_indicator_value(value);
    }

    /// Fills the current indicator over `length` bytes from `start`,
    /// `SCI_INDICATORFILLRANGE`.
    pub fn indicator_fill_range(&self, start: i32, length: i32) {
        self.control.indicator_fill(start, length);
    }

    /// Clears the current indicator over `length` bytes from `start`,
    /// `SCI_INDICATORCLEARRANGE`.
    pub fn indicator_clear_range(&self, start: i32, length: i32) {
        self.control.indicator_clear(start, length);
    }

    // Annotations.

    /// Sets the annotation text for `line`; an empty string removes it,
    /// `SCI_ANNOTATIONSETTEXT`.
    pub fn set_annotation_text(&self, line: i32, text: &str) {
        self.control
            .annotation_set_text(line, &encode_c_string(text));
    }

    /// Sets how annotations are shown, `SCI_ANNOTATIONSETVISIBLE`.
    pub fn set_annotation_visible(&self, visible: AnnotationVisible) {
        self.control.annotation_set_visible(visible as i32);
    }

    // Autocompletion and call tips.

    /// Shows the autocompletion list `list`, with `len_entered` characters
    /// already typed, `SCI_AUTOCSHOW`.
    pub fn auto_show(&self, len_entered: i32, list: &str) {
        self.control.auto_show(len_entered, &encode_c_string(list));
    }

    /// Cancels autocompletion, `SCI_AUTOCCANCEL`.
    pub fn auto_cancel(&self) {
        self.control.auto_cancel();
    }

    /// Shows a call tip at `position`, `SCI_CALLTIPSHOW`.
    pub fn call_tip_show(&self, position: i32, text: &str) {
        self.control.call_tip_show(position, &encode_c_string(text));
    }

    /// Cancels the call tip, `SCI_CALLTIPCANCEL`.
    pub fn call_tip_cancel(&self) {
        self.control.call_tip_cancel();
    }

    // Navigation and the container lexer.

    /// Scrolls to `line`, `SCI_GOTOLINE`.
    pub fn goto_line(&self, line: i32) {
        self.control.goto_line(line);
    }

    /// Returns the position of the brace matching the one at `position`, or
    /// `None` when there is none, `SCI_BRACEMATCH`.
    pub fn brace_match(&self, position: usize) -> Option<usize> {
        let position = i32::try_from(position).ok()?;
        self.control.brace_match(position)
    }

    /// Selects the container lexer (`SCI_SETILEXER(NULL)`), so Scintilla asks
    /// for styles with `SCN_STYLENEEDED` instead of running a built-in lexer.
    pub fn set_container_lexer(&self) {
        self.control.set_ilexer_container();
    }

    /// The byte position up to which styles have been applied,
    /// `SCI_GETENDSTYLED`.
    pub fn end_styled(&self) -> usize {
        self.control.end_styled()
    }

    /// The line containing `position`, `SCI_LINEFROMPOSITION`.
    pub fn line_from_position(&self, position: i32) -> i32 {
        self.control.line_from_position(position)
    }

    /// The first byte of `line`, `SCI_POSITIONFROMLINE`.
    pub fn position_from_line(&self, line: i32) -> i32 {
        self.control.position_from_line(line)
    }

    /// Paints `styles` from byte `start` without touching the lexer.
    ///
    /// `styles` holds one style number per byte. Consecutive equal numbers are
    /// coalesced into a single `SCI_SETSTYLING` run. The caller drives this
    /// from [`crate::Scn::StyleNeeded`].
    pub fn apply_styles(&self, start: usize, styles: &[u8]) {
        if styles.is_empty() {
            return;
        }
        let Ok(start) = i32::try_from(start) else {
            return;
        };
        self.control.start_styling(start);
        for (length, style) in style_runs(styles) {
            let Ok(length) = i32::try_from(length) else {
                return;
            };
            self.control.set_styling(length, style as i32);
        }
    }

    /// Selects the container lexer and paints `styles` from byte `start`.
    ///
    /// A convenience for an initial full style; the incremental path calls
    /// [`Scintilla::set_container_lexer`] once and then
    /// [`Scintilla::apply_styles`] per notification.
    pub fn set_container_lexer_and_styles(&self, start: usize, styles: &[u8]) {
        self.control.set_ilexer_container();
        self.apply_styles(start, styles);
    }

    // Margins, selection and caret colours.

    /// Copies `STYLE_DEFAULT` over every style, `SCI_STYLECLEARALL`.
    ///
    /// The themed palette calls this so the whole viewport — the area beyond
    /// the document and the margins — takes the theme background instead of
    /// Scintilla's default white.
    pub fn style_clear_all(&self) {
        self.control.style_clear_all();
    }

    /// Sets a margin's background colour, `SCI_SETMARGINBACKN`.
    pub fn set_margin_back(&self, margin: i32, color: Color) {
        self.control.margin_back(margin, color.to_scintilla());
    }

    /// Sets the selection foreground colour, `SCI_SETSELFORE`.
    pub fn set_selection_fore(&self, color: Color) {
        self.control.selection_fore(color.to_scintilla());
    }

    /// Sets the selection background colour, `SCI_SETSELBACK`.
    pub fn set_selection_back(&self, color: Color) {
        self.control.selection_back(color.to_scintilla());
    }

    /// Sets the caret colour, `SCI_SETCARETFORE`.
    pub fn set_caret_fore(&self, color: Color) {
        self.control.caret_fore(color.to_scintilla());
    }
}
