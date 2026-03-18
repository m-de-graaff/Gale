//! Integration tests for rich diagnostic rendering.

use galex::diagnostic::DiagnosticRenderer;
use galex::error::LexError;
use galex::span::{FileTable, Span};
use std::path::PathBuf;

fn renderer_with_file<'a>(source: &'a str, file_table: &'a FileTable) -> DiagnosticRenderer<'a> {
    DiagnosticRenderer::new(source, file_table, false)
}

// ── Single error rendering ─────────────────────────────────────────────

#[test]
fn single_unterminated_string() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("app/page.gx"));

    let source = "let name = \"hello";
    let r = renderer_with_file(source, &ft);

    let err = LexError::UnterminatedString {
        span: Span::new(0, 12, 17, 1, 13),
    };
    let out = r.render(&err);

    // Must contain: error code, message, file path, line number, source, caret, hint
    assert!(out.contains("error[GX0001]"));
    assert!(out.contains("unterminated string literal"));
    assert!(out.contains("--> app/page.gx:1:13"));
    assert!(out.contains("1 |"));
    assert!(out.contains("let name = \"hello"));
    assert!(out.contains("string started here but never closed"));
}

// ── Error on a specific line ───────────────────────────────────────────

#[test]
fn error_on_line_3_of_multiline_source() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("src/main.gx"));

    let source = "let a = 1\nlet b = 2\nlet c = ~\nlet d = 4";
    let r = renderer_with_file(source, &ft);

    let err = LexError::UnexpectedCharacter {
        span: Span::new(0, 28, 29, 3, 9),
        ch: '~',
    };
    let out = r.render(&err);

    assert!(out.contains("error[GX0006]"));
    assert!(out.contains("--> src/main.gx:3:9"));
    assert!(out.contains("3 |"));
    assert!(out.contains("let c = ~"));
    assert!(out.contains("this character is not valid in GaleX"));
}

// ── Multiple errors ────────────────────────────────────────────────────

#[test]
fn multiple_errors_rendered_together() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("test.gx"));

    let source = "let ~ = \"hello\nlet x = 0x";
    let r = renderer_with_file(source, &ft);

    let errors = vec![
        LexError::UnexpectedCharacter {
            span: Span::new(0, 4, 5, 1, 5),
            ch: '~',
        },
        LexError::UnterminatedString {
            span: Span::new(0, 8, 14, 1, 9),
        },
    ];

    let out = r.render_all(&errors);

    // Both error codes should appear
    assert!(out.contains("GX0006"));
    assert!(out.contains("GX0001"));

    // Count the number of error headers
    let error_count = out.matches("error[").count();
    assert_eq!(error_count, 2, "should render exactly 2 errors");
}

// ── Multi-character span gets tilde underline ──────────────────────────

#[test]
fn multi_char_span_uses_tilde_underline() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("t.gx"));

    let source = "let x = \"unterminated";
    let r = renderer_with_file(source, &ft);

    // Span covers 13 chars: "unterminated (col 9 to 21)
    let err = LexError::UnterminatedString {
        span: Span::new(0, 8, 21, 1, 9),
    };
    let out = r.render(&err);

    // Should use tildes, not a single caret
    assert!(
        out.contains("~~~"),
        "multi-char span should use tildes: {}",
        out
    );
}

#[test]
fn single_char_span_uses_caret() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("t.gx"));

    let source = "let ~ x";
    let r = renderer_with_file(source, &ft);

    let err = LexError::UnexpectedCharacter {
        span: Span::new(0, 4, 5, 1, 5),
        ch: '~',
    };
    let out = r.render(&err);

    // Should have a caret, not tildes
    assert!(
        out.contains("^ this character"),
        "single char should use caret: {}",
        out
    );
}

// ── Color mode ─────────────────────────────────────────────────────────

#[test]
fn color_mode_includes_ansi_codes() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("c.gx"));

    let source = "let ~ x";
    let r = DiagnosticRenderer::new(source, &ft, true); // color ON

    let err = LexError::UnexpectedCharacter {
        span: Span::new(0, 4, 5, 1, 5),
        ch: '~',
    };
    let out = r.render(&err);

    assert!(
        out.contains("\x1b["),
        "color mode should include ANSI escape codes"
    );
}

#[test]
fn no_color_mode_excludes_ansi_codes() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("c.gx"));

    let source = "let ~ x";
    let r = DiagnosticRenderer::new(source, &ft, false); // color OFF

    let err = LexError::UnexpectedCharacter {
        span: Span::new(0, 4, 5, 1, 5),
        ch: '~',
    };
    let out = r.render(&err);

    assert!(
        !out.contains("\x1b["),
        "no-color mode should not include ANSI codes"
    );
}

// ── Edge cases ─────────────────────────────────────────────────────────

#[test]
fn error_at_line_1_col_1() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("e.gx"));

    let source = "~rest";
    let r = renderer_with_file(source, &ft);

    let err = LexError::UnexpectedCharacter {
        span: Span::new(0, 0, 1, 1, 1),
        ch: '~',
    };
    let out = r.render(&err);

    assert!(out.contains("--> e.gx:1:1"));
    assert!(out.contains("1 |"));
    assert!(out.contains("~rest"));
}

#[test]
fn error_at_end_of_file() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("eof.gx"));

    let source = "let x = `unclosed";
    let r = renderer_with_file(source, &ft);

    let err = LexError::UnterminatedTemplateLiteral {
        span: Span::new(0, 8, 17, 1, 9),
    };
    let out = r.render(&err);

    assert!(out.contains("GX0002"));
    assert!(out.contains("template literal started here but never closed"));
}

// ── Full end-to-end: lex and render ────────────────────────────────────

#[test]
fn end_to_end_lex_then_render() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("demo.gx"));

    let source = "let x = \"unterminated\nlet y = 0b\nlet z = 1";
    let result = galex::lex(source, 0);

    assert!(result.has_errors());

    let r = DiagnosticRenderer::new(source, &ft, false);
    let out = r.render_all(&result.errors);

    // Should contain at least one formatted error
    assert!(out.contains("error["));
    assert!(out.contains("-->"));
    // Should contain some source context
    assert!(
        out.contains("let x =") || out.contains("let y ="),
        "should show source context"
    );
}

// ── All error types render without panic ───────────────────────────────

#[test]
fn all_error_types_render_successfully() {
    let mut ft = FileTable::new();
    ft.add_file(PathBuf::from("all.gx"));

    let source = "abcdefghijklmnopqrstuvwxyz";
    let r = renderer_with_file(source, &ft);

    let errors: Vec<LexError> = vec![
        LexError::UnexpectedCharacter {
            span: Span::new(0, 0, 1, 1, 1),
            ch: '~',
        },
        LexError::UnterminatedString {
            span: Span::new(0, 0, 5, 1, 1),
        },
        LexError::UnterminatedTemplateLiteral {
            span: Span::new(0, 0, 5, 1, 1),
        },
        LexError::UnterminatedBlockComment {
            span: Span::new(0, 0, 5, 1, 1),
        },
        LexError::UnterminatedRegex {
            span: Span::new(0, 0, 5, 1, 1),
        },
        LexError::InvalidEscapeSequence {
            span: Span::new(0, 0, 2, 1, 1),
            sequence: 'q',
        },
        LexError::InvalidNumberLiteral {
            span: Span::new(0, 0, 3, 1, 1),
            reason: "expected hex digits".into(),
        },
    ];

    for err in &errors {
        let out = r.render(err);
        assert!(
            out.contains(&format!("error[{}]", err.error_code())),
            "error {} should render its code",
            err.error_code()
        );
        assert!(
            out.contains(err.hint()),
            "error {} should render its hint",
            err.error_code()
        );
    }
}
