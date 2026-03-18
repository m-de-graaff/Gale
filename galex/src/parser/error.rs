//! Parser error types.

use crate::span::Span;
use crate::token::Token;

/// An error encountered during parsing.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub kind: ParseErrorKind,
}

/// Specific kind of parse error.
#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    /// Expected a specific token, found something else.
    UnexpectedToken { expected: String, found: String },
    /// Reached end of file unexpectedly.
    UnexpectedEof { expected: String },
    /// Generic invalid expression.
    InvalidExpression,
    /// Generic invalid statement.
    InvalidStatement,
    /// Generic invalid declaration.
    InvalidDeclaration,
    /// Template parsing error.
    InvalidTemplate,
}

impl ParseError {
    pub fn unexpected(expected: &str, found: &Token, span: Span) -> Self {
        Self {
            message: format!("expected {expected}, found {}", found.kind_str()),
            span,
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                found: found.kind_str().to_string(),
            },
        }
    }

    pub fn unexpected_eof(expected: &str, span: Span) -> Self {
        Self {
            message: format!("unexpected end of file, expected {expected}"),
            span,
            kind: ParseErrorKind::UnexpectedEof {
                expected: expected.to_string(),
            },
        }
    }

    pub fn invalid_expr(msg: &str, span: Span) -> Self {
        Self {
            message: msg.to_string(),
            span,
            kind: ParseErrorKind::InvalidExpression,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl crate::errors::IntoDiagnostic for ParseError {
    fn into_diagnostic(self) -> crate::errors::Diagnostic {
        let code = match &self.kind {
            ParseErrorKind::UnexpectedToken { .. } => &crate::errors::codes::GX0100,
            ParseErrorKind::UnexpectedEof { .. } => &crate::errors::codes::GX0101,
            ParseErrorKind::InvalidExpression => &crate::errors::codes::GX0115,
            ParseErrorKind::InvalidStatement => &crate::errors::codes::GX0100,
            ParseErrorKind::InvalidDeclaration => &crate::errors::codes::GX0100,
            ParseErrorKind::InvalidTemplate => &crate::errors::codes::GX0111,
        };
        crate::errors::Diagnostic::with_message(code, self.message, self.span)
    }
}
