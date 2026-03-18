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

/// Helper: convert a DiagnosticLevel to an LSP severity.
fn level_to_severity(level: crate::errors::DiagnosticLevel) -> DiagnosticSeverity {
    match level {
        crate::errors::DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        crate::errors::DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        crate::errors::DiagnosticLevel::Hint => DiagnosticSeverity::HINT,
    }
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
        let code_ref = err.error_code_ref();
        diags.push(Diagnostic {
            range: span_to_range(&err.span(), source),
            severity: Some(level_to_severity(code_ref.level)),
            code: Some(lsp_types::NumberOrString::String(code_ref.as_str())),
            source: Some("galex".into()),
            message: err.message().to_string(),
            ..Default::default()
        });
    }

    for err in parse_errors {
        // ParseError doesn't have a code field — derive from kind
        let code_ref = match &err.kind {
            crate::parser::error::ParseErrorKind::UnexpectedToken { .. } => {
                &crate::errors::codes::GX0100
            }
            crate::parser::error::ParseErrorKind::UnexpectedEof { .. } => {
                &crate::errors::codes::GX0101
            }
            crate::parser::error::ParseErrorKind::InvalidExpression => {
                &crate::errors::codes::GX0115
            }
            crate::parser::error::ParseErrorKind::InvalidStatement => &crate::errors::codes::GX0100,
            crate::parser::error::ParseErrorKind::InvalidDeclaration => {
                &crate::errors::codes::GX0100
            }
            crate::parser::error::ParseErrorKind::InvalidTemplate => &crate::errors::codes::GX0111,
        };
        diags.push(Diagnostic {
            range: span_to_range(&err.span, source),
            severity: Some(level_to_severity(code_ref.level)),
            code: Some(lsp_types::NumberOrString::String(code_ref.as_str())),
            source: Some("galex".into()),
            message: err.message.clone(),
            ..Default::default()
        });
    }

    for err in type_errors {
        diags.push(Diagnostic {
            range: span_to_range(&err.span, source),
            severity: Some(level_to_severity(err.code.level)),
            code: Some(lsp_types::NumberOrString::String(err.code.as_str())),
            source: Some("galex".into()),
            message: format!("{err}"),
            ..Default::default()
        });
    }

    for warn in lint_warnings {
        diags.push(Diagnostic {
            range: span_to_range(&warn.span, source),
            severity: Some(level_to_severity(warn.code.level)),
            code: Some(lsp_types::NumberOrString::String(warn.code.as_str())),
            source: Some("galex".into()),
            message: warn.message.clone(),
            ..Default::default()
        });
    }

    diags
}

/// Convert unified [`crate::errors::Diagnostic`] values to LSP Diagnostic objects.
///
/// This works with the new unified error system where all compiler phases
/// produce `Diagnostic` values with stable GX error codes.
pub fn from_diagnostics(
    diagnostics: &[crate::errors::Diagnostic],
    source: &str,
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|d| lsp_types::Diagnostic {
            range: span_to_range(&d.span, source),
            severity: Some(level_to_severity(d.code.level)),
            code: Some(lsp_types::NumberOrString::String(d.code.as_str())),
            source: Some("galex".into()),
            message: d.message.clone(),
            ..Default::default()
        })
        .collect()
}
