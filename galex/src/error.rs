//! Lexer error types with source spans and diagnostic metadata.

use crate::span::Span;
use std::fmt;

/// An error encountered during lexing.
///
/// The lexer accumulates errors and attempts to recover, so multiple
/// errors may be reported for a single source file.
#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    /// A string literal was not closed before end of line or file.
    UnterminatedString { span: Span },
    /// A template literal (backtick) was not closed.
    UnterminatedTemplateLiteral { span: Span },
    /// A block comment `/* */` was not closed.
    UnterminatedBlockComment { span: Span },
    /// A regex literal `/pattern/` was not closed.
    UnterminatedRegex { span: Span },
    /// An invalid escape sequence in a string or template literal.
    InvalidEscapeSequence { span: Span, sequence: char },
    /// An invalid number literal (e.g. `0xZZ`).
    InvalidNumberLiteral { span: Span, reason: String },
    /// An unexpected character that doesn't start any valid token.
    UnexpectedCharacter { span: Span, ch: char },
}

impl LexError {
    /// Get the span where this error occurred.
    pub fn span(&self) -> &Span {
        match self {
            LexError::UnterminatedString { span } => span,
            LexError::UnterminatedTemplateLiteral { span } => span,
            LexError::UnterminatedBlockComment { span } => span,
            LexError::UnterminatedRegex { span } => span,
            LexError::InvalidEscapeSequence { span, .. } => span,
            LexError::InvalidNumberLiteral { span, .. } => span,
            LexError::UnexpectedCharacter { span, .. } => span,
        }
    }

    /// Stable error code for this error kind (e.g. `"GX0001"`).
    ///
    /// Maps to the canonical codes defined in [`crate::errors::codes`].
    pub fn error_code(&self) -> &'static str {
        match self {
            LexError::UnterminatedString { .. } => "GX0001",
            LexError::UnterminatedTemplateLiteral { .. } => "GX0002",
            LexError::UnterminatedBlockComment { .. } => "GX0003",
            LexError::InvalidEscapeSequence { .. } => "GX0004",
            LexError::InvalidNumberLiteral { .. } => "GX0005",
            LexError::UnexpectedCharacter { .. } => "GX0006",
            LexError::UnterminatedRegex { .. } => "GX0009",
        }
    }

    /// Get the corresponding [`ErrorCode`](crate::errors::ErrorCode) from `codes.rs`.
    pub fn error_code_ref(&self) -> &'static crate::errors::ErrorCode {
        match self {
            LexError::UnterminatedString { .. } => &crate::errors::codes::GX0001,
            LexError::UnterminatedTemplateLiteral { .. } => &crate::errors::codes::GX0002,
            LexError::UnterminatedBlockComment { .. } => &crate::errors::codes::GX0003,
            LexError::InvalidEscapeSequence { .. } => &crate::errors::codes::GX0004,
            LexError::InvalidNumberLiteral { .. } => &crate::errors::codes::GX0005,
            LexError::UnexpectedCharacter { .. } => &crate::errors::codes::GX0006,
            LexError::UnterminatedRegex { .. } => &crate::errors::codes::GX0009,
        }
    }

    /// Short human-readable description of the error.
    pub fn message(&self) -> String {
        match self {
            LexError::UnexpectedCharacter { ch, .. } => {
                format!("unexpected character '{}'", ch.escape_debug())
            }
            LexError::UnterminatedString { .. } => "unterminated string literal".into(),
            LexError::UnterminatedTemplateLiteral { .. } => "unterminated template literal".into(),
            LexError::UnterminatedBlockComment { .. } => "unterminated block comment".into(),
            LexError::UnterminatedRegex { .. } => "unterminated regex literal".into(),
            LexError::InvalidEscapeSequence { sequence, .. } => {
                format!("invalid escape sequence '\\{}'", sequence.escape_debug())
            }
            LexError::InvalidNumberLiteral { reason, .. } => {
                format!("invalid number literal: {}", reason)
            }
        }
    }

    /// Contextual hint displayed below the caret in diagnostic output.
    pub fn hint(&self) -> &'static str {
        match self {
            LexError::UnexpectedCharacter { .. } => "this character is not valid in GaleX",
            LexError::UnterminatedString { .. } => "string started here but never closed",
            LexError::UnterminatedTemplateLiteral { .. } => {
                "template literal started here but never closed"
            }
            LexError::UnterminatedBlockComment { .. } => {
                "block comment started here but never closed"
            }
            LexError::UnterminatedRegex { .. } => "regex started here but never closed",
            LexError::InvalidEscapeSequence { .. } => {
                "valid escapes: \\n \\t \\r \\\\ \\\" \\' \\` \\{ \\$ \\0 \\xHH \\u{HHHH}"
            }
            LexError::InvalidNumberLiteral { .. } => "expected valid digits for this number base",
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}]: {}", self.error_code(), self.message())
    }
}

impl std::error::Error for LexError {}

impl crate::errors::IntoDiagnostic for LexError {
    fn into_diagnostic(self) -> crate::errors::Diagnostic {
        let code = self.error_code_ref();
        let message = self.message();
        let span = *self.span();
        crate::errors::Diagnostic::with_message(code, message, span).with_hint(self.hint())
    }
}

/// The result of lexing a source file.
pub struct LexResult {
    /// The token stream (may be partial if errors occurred).
    pub tokens: Vec<crate::token::TokenWithSpan>,
    /// Errors encountered during lexing.
    pub errors: Vec<LexError>,
}

impl LexResult {
    /// Returns `true` if lexing completed without errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns `true` if any errors were encountered.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_error_codes_are_unique() {
        let errors: Vec<LexError> = vec![
            LexError::UnexpectedCharacter {
                span: Span::dummy(),
                ch: '~',
            },
            LexError::UnterminatedString {
                span: Span::dummy(),
            },
            LexError::UnterminatedTemplateLiteral {
                span: Span::dummy(),
            },
            LexError::UnterminatedBlockComment {
                span: Span::dummy(),
            },
            LexError::UnterminatedRegex {
                span: Span::dummy(),
            },
            LexError::InvalidEscapeSequence {
                span: Span::dummy(),
                sequence: 'q',
            },
            LexError::InvalidNumberLiteral {
                span: Span::dummy(),
                reason: "bad".into(),
            },
        ];

        let codes: Vec<&str> = errors.iter().map(|e| e.error_code()).collect();
        let mut unique = codes.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(codes.len(), unique.len(), "error codes must be unique");
    }

    #[test]
    fn error_display_includes_code_and_message() {
        let err = LexError::UnterminatedString {
            span: Span::new(0, 10, 15, 2, 5),
        };
        let s = format!("{}", err);
        assert!(s.contains("GX0001"));
        assert!(s.contains("unterminated string literal"));
    }

    #[test]
    fn every_error_has_nonempty_hint() {
        let errors: Vec<LexError> = vec![
            LexError::UnexpectedCharacter {
                span: Span::dummy(),
                ch: '~',
            },
            LexError::UnterminatedString {
                span: Span::dummy(),
            },
            LexError::UnterminatedTemplateLiteral {
                span: Span::dummy(),
            },
            LexError::UnterminatedBlockComment {
                span: Span::dummy(),
            },
            LexError::UnterminatedRegex {
                span: Span::dummy(),
            },
            LexError::InvalidEscapeSequence {
                span: Span::dummy(),
                sequence: 'q',
            },
            LexError::InvalidNumberLiteral {
                span: Span::dummy(),
                reason: "bad".into(),
            },
        ];
        for err in &errors {
            assert!(
                !err.hint().is_empty(),
                "{} should have a hint",
                err.error_code()
            );
        }
    }
}
