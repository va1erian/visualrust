//! The safe [`Scintilla`] handle.
//!
//! It owns a control (and the hidden host window that parents it) and exposes
//! typed methods instead of raw `SCI_*` messages. Decoding and pointer handling
//! live in [`crate::sys`] and [`crate::codec`].

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

    /// Selects the container lexer (`SCI_SETILEXER(NULL)`) and styles
    /// `styles[start..start + styles.len()]` in the same call.
    ///
    /// `styles` holds one style number per byte. Consecutive equal numbers are
    /// coalesced into a single `SCI_SETSTYLING` run. The caller is expected to
    /// drive this from [`crate::Scn::StyleNeeded`].
    pub fn set_container_lexer_and_styles(&self, start: usize, styles: &[u8]) {
        self.control.set_ilexer_container();
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
}
