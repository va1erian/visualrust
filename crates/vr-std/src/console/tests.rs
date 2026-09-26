//! Unit tests for the console prompt logic, driven by in-memory buffers.

use std::io::Cursor;

use super::*;

#[test]
fn prompt_writes_the_question_and_returns_the_typed_text() {
    let mut input = Cursor::new(b"Ada\n".to_vec());
    let mut output = Vec::new();
    let line = prompt(&mut input, &mut output, "Name? ").expect("reads");
    assert_eq!(line, "Ada");
    assert_eq!(output, b"Name? ");
}

#[test]
fn prompt_strips_a_windows_line_ending() {
    let mut input = Cursor::new(b"Grace\r\n".to_vec());
    let mut output = Vec::new();
    assert_eq!(prompt(&mut input, &mut output, "").expect("reads"), "Grace");
}

#[test]
fn prompt_at_end_of_input_returns_an_empty_line() {
    let mut input = Cursor::new(Vec::new());
    let mut output = Vec::new();
    assert_eq!(prompt(&mut input, &mut output, "> ").expect("reads"), "");
    assert_eq!(output, b"> ");
}
