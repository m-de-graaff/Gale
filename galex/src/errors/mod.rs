//! Unified error system for the GaleX compiler.
//!
//! All 331 error codes are defined in [`codes`]. Every compiler phase
//! produces [`Diagnostic`] values that carry a stable error code,
//! source span, formatted message, and optional help text.

pub mod codes;

use crate::span::Span;
use std::fmt;

// ── Diagnostic level ───────────────────────────────────────────────────

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticLevel {
    /// Blocks compilation.
    Error,
    /// Compiles but suspicious.
    Warning,
    /// Suggestion only.
    Hint,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "error"),
            DiagnosticLevel::Warning => write!(f, "warning"),
            DiagnosticLevel::Hint => write!(f, "hint"),
        }
    }
}

// ── Error code ─────────────────────────────────────────────────────────

/// A stable GaleX error code (GX0001–GX2099).
///
/// Codes are assigned once and never change meaning. Each code belongs
/// to a subsystem range and has a fixed severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    /// Numeric code (1–2099).
    pub code: u16,
    /// Fixed severity level.
    pub level: DiagnosticLevel,
    /// One-line message template (may contain `{}` placeholders).
    pub message: &'static str,
    /// Default hint shown below the caret in diagnostic output.
    pub hint: &'static str,
}

impl ErrorCode {
    /// Create a new error code. Used as a `const fn` for static definitions.
    pub const fn new(
        code: u16,
        level: DiagnosticLevel,
        message: &'static str,
        hint: &'static str,
    ) -> Self {
        Self {
            code,
            level,
            message,
            hint,
        }
    }

    /// Format the code as a string like `"GX0042"`.
    pub fn as_str(&self) -> String {
        format!("GX{:04}", self.code)
    }

    /// Returns the subsystem name for this error code's range.
    pub fn subsystem(&self) -> &'static str {
        match self.code {
            1..=99 => "lexer",
            100..=299 => "parser",
            300..=499 => "types",
            500..=599 => "boundary",
            600..=699 => "guard",
            700..=799 => "template",
            800..=899 => "module",
            900..=999 => "action",
            1000..=1099 => "store",
            1100..=1199 => "env",
            1200..=1299 => "routing",
            1300..=1399 => "middleware",
            1400..=1499 => "head",
            1500..=1599 => "form",
            1600..=1699 => "reactivity",
            1700..=1799 => "lint",
            1800..=1899 => "build",
            1900..=1999 => "runtime",
            2000..=2099 => "package",
            _ => "unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GX{:04}", self.code)
    }
}

// ── Diagnostic ─────────────────────────────────────────────────────────

/// A compiler diagnostic with a stable error code, source location,
/// and human-readable message.
///
/// Every error, warning, and hint produced by the GaleX compiler is
/// represented as a `Diagnostic`.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The stable error code (e.g., GX0042).
    pub code: &'static ErrorCode,
    /// Formatted message (template filled with specific details).
    pub message: String,
    /// Source location where the error occurred.
    pub span: Span,
    /// Contextual hint shown below the caret (overrides the code's default if set).
    pub hint: Option<String>,
    /// Multi-line help/suggestion text (shown as a `help:` block).
    pub help: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic with the code's default message.
    pub fn new(code: &'static ErrorCode, span: Span) -> Self {
        Self {
            code,
            message: code.message.to_string(),
            span,
            hint: None,
            help: None,
        }
    }

    /// Create a new diagnostic with a custom formatted message.
    pub fn with_message(code: &'static ErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            hint: None,
            help: None,
        }
    }

    /// Add a contextual hint (overrides the code's default hint).
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add a multi-line help suggestion.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Get the effective hint text (custom or default from code).
    pub fn effective_hint(&self) -> &str {
        self.hint.as_deref().unwrap_or(self.code.hint)
    }

    /// Get the severity level from the error code.
    pub fn level(&self) -> DiagnosticLevel {
        self.code.level
    }

    /// Returns true if this diagnostic blocks compilation.
    pub fn is_error(&self) -> bool {
        self.code.level == DiagnosticLevel::Error
    }

    /// Returns true if this is a warning.
    pub fn is_warning(&self) -> bool {
        self.code.level == DiagnosticLevel::Warning
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]: {}", self.code.level, self.code, self.message)
    }
}

// ── Conversion trait ───────────────────────────────────────────────────

/// Trait for converting phase-specific errors into unified [`Diagnostic`]s.
pub trait IntoDiagnostic {
    /// Convert this error into a `Diagnostic`.
    fn into_diagnostic(self) -> Diagnostic;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_formatting() {
        let code = ErrorCode::new(1, DiagnosticLevel::Error, "test", "hint");
        assert_eq!(code.as_str(), "GX0001");
        assert_eq!(format!("{}", code), "GX0001");
    }

    #[test]
    fn error_code_formatting_large() {
        let code = ErrorCode::new(2099, DiagnosticLevel::Warning, "test", "hint");
        assert_eq!(code.as_str(), "GX2099");
    }

    #[test]
    fn subsystem_ranges() {
        assert_eq!(
            ErrorCode::new(1, DiagnosticLevel::Error, "", "").subsystem(),
            "lexer"
        );
        assert_eq!(
            ErrorCode::new(100, DiagnosticLevel::Error, "", "").subsystem(),
            "parser"
        );
        assert_eq!(
            ErrorCode::new(300, DiagnosticLevel::Error, "", "").subsystem(),
            "types"
        );
        assert_eq!(
            ErrorCode::new(500, DiagnosticLevel::Error, "", "").subsystem(),
            "boundary"
        );
        assert_eq!(
            ErrorCode::new(1700, DiagnosticLevel::Warning, "", "").subsystem(),
            "lint"
        );
    }

    #[test]
    fn diagnostic_display() {
        use crate::span::Span;
        let code = &super::codes::GX0001;
        let diag = Diagnostic::new(code, Span::dummy());
        let s = format!("{}", diag);
        assert!(s.contains("error[GX0001]"));
        assert!(s.contains("Unterminated string literal"));
    }

    #[test]
    fn diagnostic_with_custom_hint() {
        use crate::span::Span;
        let code = &super::codes::GX0001;
        let diag = Diagnostic::new(code, Span::dummy()).with_hint("custom hint");
        assert_eq!(diag.effective_hint(), "custom hint");
    }

    #[test]
    fn diagnostic_default_hint() {
        use crate::span::Span;
        let code = &super::codes::GX0001;
        let diag = Diagnostic::new(code, Span::dummy());
        assert_eq!(diag.effective_hint(), code.hint);
    }
}
