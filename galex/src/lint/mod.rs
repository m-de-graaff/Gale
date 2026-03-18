//! GaleX linter — static analysis rules for .gx files.

pub mod rules;

use crate::ast::Program;
use crate::span::Span;

/// A lint warning or error.
#[derive(Debug, Clone)]
pub struct LintWarning {
    /// Rule name (e.g. "unused-signal", "missing-alt").
    pub rule: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Source location.
    pub span: Span,
    /// Severity level.
    pub severity: Severity,
}

/// Lint severity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

impl std::fmt::Display for LintWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(
            f,
            "{level}[{}] ({}:{}): {}",
            self.rule, self.span.line, self.span.col, self.message
        )
    }
}

/// Run all lint rules on a program and return warnings.
pub fn lint_program(program: &Program) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    rules::check_unused_signals(program, &mut warnings);
    rules::check_unused_derives(program, &mut warnings);
    rules::check_empty_blocks(program, &mut warnings);
    rules::check_missing_key_on_each(program, &mut warnings);
    rules::check_missing_alt_on_img(program, &mut warnings);
    rules::check_unreachable_after_return(program, &mut warnings);
    warnings
}
