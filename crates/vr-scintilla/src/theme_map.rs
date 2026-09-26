//! Mapping win32ui [`Theme`] tokens onto Scintilla styles and chrome.
//!
//! Scintilla owns its own style table; there is no `Themed` hook for a foreign
//! control, so the host applies a palette explicitly at creation and whenever
//! the application switches theme. The mapping is deliberately derived from a
//! handful of semantic tokens (`accent`, `warning`, `danger`, `text*`) so light
//! and dark stay coherent rather than defining a second, drifting palette.

use win32ui::Theme;

use vr_syntax::{STYLE_COUNT, StyleKind};

use crate::types::{STYLE_DEFAULT, STYLE_LINENUMBER};
use crate::{Color, Scintilla};

/// The colours and emphasis one Scintilla style is drawn with.
#[derive(Clone, Copy)]
struct StyleSpec {
    fore: Color,
    back: Color,
    bold: bool,
    italic: bool,
}

/// The resolved Scintilla palette for one theme.
pub(crate) struct Palette {
    styles: [StyleSpec; STYLE_COUNT as usize],
    margin_back: Color,
    margin_fore: Color,
    selection_back: Color,
    selection_fore: Color,
    caret: Color,
}

/// Converts a win32ui token colour to the wrapper's colour type.
const fn color(token: win32ui::Color) -> Color {
    Color::rgb(token.r, token.g, token.b)
}

/// Builds the palette for `theme`.
pub(crate) fn palette(theme: &Theme) -> Palette {
    let back = color(theme.input_background);
    let text = color(theme.text);
    let secondary = color(theme.text_secondary);
    let accent = color(theme.accent);
    // A string in amber and a number in the danger hue read clearly apart from
    // the accent keywords and grey comments, in both palettes.
    let string = color(theme.warning);
    let literal = color(theme.danger);

    let style = |fore: Color, bold: bool, italic: bool| StyleSpec {
        fore,
        back,
        bold,
        italic,
    };

    let mut styles = [style(text, false, false); STYLE_COUNT as usize];
    styles[StyleKind::Default as usize] = style(text, false, false);
    styles[StyleKind::Keyword as usize] = style(accent, true, false);
    styles[StyleKind::Function as usize] = style(text, true, false);
    styles[StyleKind::Comment as usize] = style(secondary, false, true);
    styles[StyleKind::DocComment as usize] = style(secondary, false, true);
    styles[StyleKind::String as usize] = style(string, false, false);
    styles[StyleKind::Number as usize] = style(literal, false, false);
    styles[StyleKind::Operator as usize] = style(secondary, false, false);
    styles[StyleKind::Link as usize] = style(accent, true, false);
    styles[StyleKind::Error as usize] = style(literal, true, false);

    Palette {
        styles,
        margin_back: color(theme.background),
        margin_fore: secondary,
        selection_back: color(theme.selection),
        selection_fore: text,
        caret: text,
    }
}

/// Applies `theme`'s palette to `editor`: every Dyon style, the line-number
/// margin, the selection and the caret.
pub(crate) fn apply(editor: &Scintilla, theme: &Theme) {
    let palette = palette(theme);
    // `STYLE_DEFAULT` (32) is not a Dyon style but drives everything that is
    // not explicitly styled: the area beyond the document and the margins.
    // Seed it, copy it over all 256 styles, then override the Dyon styles.
    editor.set_style_fore(
        STYLE_DEFAULT,
        palette.styles[StyleKind::Default as usize].fore,
    );
    editor.set_style_back(
        STYLE_DEFAULT,
        palette.styles[StyleKind::Default as usize].back,
    );
    editor.style_clear_all();
    for (index, spec) in palette.styles.iter().enumerate() {
        let style = index as u8;
        editor.set_style_fore(style, spec.fore);
        editor.set_style_back(style, spec.back);
        editor.set_style_bold(style, spec.bold);
        editor.set_style_italic(style, spec.italic);
    }
    // A number margin draws its text with `STYLE_LINENUMBER`, whose background
    // must match the margin's own, not the editor's.
    editor.set_style_fore(STYLE_LINENUMBER, palette.margin_fore);
    editor.set_style_back(STYLE_LINENUMBER, palette.margin_back);
    editor.set_margin_back(0, palette.margin_back);
    editor.set_selection_fore(palette.selection_fore);
    editor.set_selection_back(palette.selection_back);
    editor.set_caret_fore(palette.caret);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_keyword_string_comments_differ() {
        let light = palette(&Theme::light());
        let dark = palette(&Theme::dark());
        let keyword = StyleKind::Keyword as usize;
        let string = StyleKind::String as usize;
        let comment = StyleKind::Comment as usize;
        for palette in [&light, &dark] {
            let k = palette.styles[keyword].fore;
            let s = palette.styles[string].fore;
            let c = palette.styles[comment].fore;
            assert_ne!(k, s, "keyword and string share a colour");
            assert_ne!(k, c, "keyword and comment share a colour");
            assert_ne!(s, c, "string and comment share a colour");
        }
        assert_ne!(light.styles[keyword].fore, dark.styles[keyword].fore);
    }

    #[test]
    fn every_style_starts_from_the_editor_background() {
        let palette = palette(&Theme::light());
        let expected = color(Theme::light().input_background);
        for spec in &palette.styles {
            assert_eq!(spec.back, expected);
        }
    }
}
