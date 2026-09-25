//! End-to-end highlighting over representative Dyon programs, driven only
//! through the public API.

use vr_syntax::{Lexer, LineState, StyleKind, StyleRun};

const SAMPLE: &str = r#"// Dyon sample
fn main() {
    mut total = 1.5e3
    /* count up
       to three */
    for i in 0..3 {
        total = total + i
    }
    if total > 0 {
        print("ok")
    } else {
        return none
    }
}
"#;

fn kind_of(runs: &[StyleRun], source: &str, needle: &str) -> StyleKind {
    let at = source.find(needle).expect("needle is present");
    runs.iter()
        .find(|run| run.start <= at && at < run.start + run.len)
        .expect("a run covers the needle")
        .kind
}

#[test]
fn sample_program_uses_every_style_category() {
    let mut lexer = Lexer::new();
    let runs = lexer.style_all(SAMPLE);

    // Runs tile the document with no gaps or overlaps.
    let mut cursor = 0;
    for run in &runs {
        assert_eq!(run.start, cursor, "runs are contiguous");
        assert!(run.len > 0);
        cursor += run.len;
    }
    assert_eq!(cursor, SAMPLE.len(), "every byte is styled");

    assert_eq!(kind_of(&runs, SAMPLE, "fn"), StyleKind::Keyword);
    assert_eq!(kind_of(&runs, SAMPLE, "main"), StyleKind::Function);
    assert_eq!(kind_of(&runs, SAMPLE, "mut"), StyleKind::Keyword);
    assert_eq!(kind_of(&runs, SAMPLE, "for"), StyleKind::Keyword);
    assert_eq!(kind_of(&runs, SAMPLE, "in 0"), StyleKind::Keyword);
    assert_eq!(kind_of(&runs, SAMPLE, "1.5e3"), StyleKind::Number);
    assert_eq!(kind_of(&runs, SAMPLE, "\"ok\""), StyleKind::String);
    assert_eq!(kind_of(&runs, SAMPLE, "none"), StyleKind::Keyword);
    assert_eq!(kind_of(&runs, SAMPLE, "// Dyon sample"), StyleKind::Comment);

    // The block comment covers its first line's newline and the closing `*/`.
    let comment = runs
        .iter()
        .find(|run| run.start == SAMPLE.find("/* count up").expect("comment"))
        .expect("comment run");
    assert_eq!(comment.kind, StyleKind::Comment);
    assert!(comment.start + comment.len > SAMPLE.find("to three */").expect("tail") + 2);
}

#[test]
fn a_doc_comment_on_its_own_line_is_distinguished() {
    let mut lexer = Lexer::new();
    let runs = lexer.style_all("/// docs\nfn f() {}\n");
    assert_eq!(runs[0].kind, StyleKind::DocComment);
}

#[test]
fn incremental_restyle_after_an_edit_matches_a_full_pass() {
    let mut lexer = Lexer::new();
    lexer.style_all(SAMPLE);

    let edited = SAMPLE.replace("total + i", "total + i + 10");
    let line = edited[..edited.find("total + i + 10").expect("edit")]
        .matches('\n')
        .count();
    let incremental = lexer.restyle_lines(&edited, line);
    assert!(
        lexer.last_scanned_lines() < lexer.line_count(),
        "an edit near the end does not rescan the whole file"
    );

    let full = Lexer::new().style_all(&edited);
    for needle in ["total + i + 10", "10", "return none", "}"] {
        assert_eq!(
            kind_of(&incremental, &edited, needle),
            kind_of(&full, &edited, needle),
            "the incremental tail matches a full pass at {needle:?}"
        );
    }
}

#[test]
fn unterminated_block_comment_leaves_the_rest_of_the_file_commented() {
    let mut lexer = Lexer::new();
    let runs = lexer.style_all("fn a() {}\n/* open\nstill open\n");
    assert_eq!(
        lexer.end_state(1),
        Some(LineState::BlockComment {
            depth: 1,
            doc: false
        })
    );
    assert_eq!(lexer.state_at_line(2), lexer.end_state(1));
    let last = runs.last().expect("a run");
    assert_eq!(last.kind, StyleKind::Comment);
}
