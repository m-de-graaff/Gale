//! Convert internal errors to LSP Diagnostic objects.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::error::LexError;
use crate::lint::LintWarning;
use crate::parser::error::ParseError;
use crate::span::Span;
use crate::types::constraint::TypeError;

/// Convert a Span to an LSP Range.
///
/// Uses the source text to compute the end line/col (the Span only
/// stores start line/col).
pub fn span_to_range(span: &Span, source: &str) -> Range {
    let start = Position {
        line: span.line.saturating_sub(1),
        character: span.col.saturating_sub(1),
    };
    let (end_line, end_col) = span.end_position(source);
    let end = Position {
        line: end_line.saturating_sub(1),
        character: end_col.saturating_sub(1),
    };
    Range { start, end }
}

/// Collect all diagnostics from lex, parse, type, and lint errors.
pub fn collect_diagnostics(
    lex_errors: &[LexError],
    parse_errors: &[ParseError],
    type_errors: &[TypeError],
    lint_warnings: &[LintWarning],
    source: &str,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for err in lex_errors {
        diags.push(Diagnostic {
            range: span_to_range(&err.span(), source),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String(err.error_code().into())),
            source: Some("galex".into()),
            message: err.message().to_string(),
            ..Default::default()
        });
    }

    for err in parse_errors {
        diags.push(Diagnostic {
            range: span_to_range(&err.span, source),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("galex".into()),
            message: err.message.clone(),
            ..Default::default()
        });
    }

    for err in type_errors {
        diags.push(Diagnostic {
            range: span_to_range(&err.span, source),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("galex".into()),
            message: format!("{err}"),
            ..Default::default()
        });
    }

    for warn in lint_warnings {
        diags.push(Diagnostic {
            range: span_to_range(&warn.span, source),
            severity: Some(match warn.severity {
                crate::lint::Severity::Warning => DiagnosticSeverity::WARNING,
                crate::lint::Severity::Error => DiagnosticSeverity::ERROR,
            }),
            code: Some(lsp_types::NumberOrString::String(warn.rule.into())),
            source: Some("galex".into()),
            message: warn.message.clone(),
            ..Default::default()
        });
    }

    diags
}
