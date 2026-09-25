//! Control round-trip tests.
//!
//! They create a real (hidden) Scintilla control. When the session cannot create
//! a window — for example a non-interactive desktop — the test prints `SKIP` and
//! passes, following win32ui's test pattern.

use vr_scintilla::{AnnotationVisible, Color, IndicatorStyle, MarginType, MarkerSymbol, Scintilla};

#[test]
fn text_round_trips_through_a_real_control() {
    let editor = match Scintilla::new() {
        Ok(editor) => editor,
        Err(reason) => {
            eprintln!("SKIP vr-scintilla control test: {reason}");
            return;
        }
    };

    assert_eq!(editor.length(), 0, "a fresh control holds no text");
    assert_eq!(editor.text().ok().as_deref(), Some(""));

    let text = "fn main() {\n    \u{2713}\n}";
    editor.set_text(text);
    assert_eq!(editor.length(), text.len());
    assert_eq!(editor.text().ok().as_deref(), Some(text));
}

#[test]
fn styling_margins_indicators_annotations_and_completion_reach_the_control() {
    let editor = match Scintilla::new() {
        Ok(editor) => editor,
        Err(reason) => {
            eprintln!("SKIP vr-scintilla control test: {reason}");
            return;
        }
    };
    editor.set_text("( )\nsecond line");

    editor.set_style_fore(0, Color::rgb(0x20, 0x80, 0x20));
    editor.set_style_back(0, Color::rgb(0xFF, 0xFF, 0xFF));
    editor.set_style_bold(0, true);
    editor.set_style_italic(0, false);
    editor.set_style_size(0, 11);

    editor.set_margin_type(0, MarginType::Number);
    editor.set_margin_width(0, 40);
    editor.set_margin_type(1, MarginType::Text);
    editor.set_margin_width(1, 80);
    editor.set_margin_text(0, "one");

    editor.define_marker(0, MarkerSymbol::Bookmark);
    assert!(editor.add_marker(0, 0) >= 0, "a marker handle is returned");
    editor.delete_marker(0, 0);

    editor.set_indicator_style(0, IndicatorStyle::Squiggle);
    editor.set_indicator_fore(0, Color::rgb(0xFF, 0x00, 0x00));
    editor.set_indicator_alpha(0, 200);
    editor.set_indicator_current(0);
    editor.set_indicator_value(1);
    editor.indicator_fill_range(0, 3);
    editor.indicator_clear_range(0, 3);

    editor.set_annotation_text(0, "note");
    editor.set_annotation_visible(AnnotationVisible::Boxed);

    editor.auto_show(0, "alpha beta");
    editor.auto_cancel();

    editor.call_tip_show(0, "fn main()");
    editor.call_tip_cancel();

    editor.goto_line(1);

    assert_eq!(
        editor.brace_match(0),
        Some(2),
        "the braces at 0 and 2 match"
    );

    editor.set_container_lexer_and_styles(0, &[1, 1, 2, 2, 2, 3]);
    assert_eq!(editor.text().ok().as_deref(), Some("( )\nsecond line"));
}
