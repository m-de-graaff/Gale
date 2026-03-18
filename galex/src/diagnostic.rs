//! Rich diagnostic rendering for lexer errors.
//!
//! Renders errors with source context, caret underlines, and optional ANSI color:
//!
//! ```text
//! error[GX0002]: unterminated string literal
//!  --> app/page.gx:12:15
//!   |
//! 12|   let name = "hello
//!   |               ^ string started here but never closed
//! ```

use crate::error::LexError;
use crate::span::FileTable;
use std::fmt::Write;

/// Renders lexer errors with source context and caret underlines.
///
/// Requires access to the original source text for line extraction.
///
/// # Example
///
/// ```
/// use galex::diagnostic::DiagnosticRenderer;
/// use galex::span::FileTable;
///
/// let source = "let x = \"hello\nlet y = 1";
/// let file_table = FileTable::new();
/// let renderer = DiagnosticRenderer::new(source, &file_table, false);
/// ```
pub struct DiagnosticRenderer<'src> {
    source: &'src str,
    file_table: &'src FileTable,
    style: Style,
    /// Byte offsets where each line starts (0-indexed into source).
    /// `line_starts[0]` is the start of line 1.
    line_starts: Vec<usize>,
}

impl<'src> DiagnosticRenderer<'src> {
    /// Create a new renderer.
    ///
    /// - `source`: the full source text that was lexed
    /// - `file_table`: maps file IDs to paths for display
    /// - `color`: whether to emit ANSI escape codes
    pub fn new(source: &'src str, file_table: &'src FileTable, color: bool) -> Self {
        let line_starts = Self::compute_line_starts(source);
        Self {
            source,
            file_table,
            style: Style { color },
            line_starts,
        }
    }

    /// Precompute the byte offset where each line starts.
    fn compute_line_starts(source: &str) -> Vec<usize> {
        let mut starts = vec![0]; // line 1 starts at offset 0
        for (i, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    /// Get the source text for a 1-based line number.
    /// Returns the line without its trailing newline.
    pub fn extract_line(&self, line: u32) -> &'src str {
        let idx = (line as usize).saturating_sub(1);
        if idx >= self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[idx];
        let end = if idx + 1 < self.line_starts.len() {
            // Up to (but not including) the \n
            self.line_starts[idx + 1].saturating_sub(1)
        } else {
            self.source.len()
        };
        // Trim trailing \r for Windows line endings
        self.source[start..end].trim_end_matches('\r')
    }

    /// Render a single error into a formatted diagnostic string.
    pub fn render(&self, error: &LexError) -> String {
        let mut out = String::new();
        let span = error.span();
        let code = error.error_code();
        let message = error.message();
        let hint = error.hint();

        // Line 1: error[GXnnnn]: message
        let _ = writeln!(
            out,
            "{}",
            self.style.red_bold(&format!("error[{}]", code)) + ": " + &self.style.bold(&message)
        );

        // Line 2:  --> file:line:col
        let location = self.file_table.format_span(span);
        let _ = writeln!(out, " {} {}", self.style.blue("-->"), location);

        // Line 3:   |
        let line_num = span.line;
        let gutter_width = digit_count(line_num);
        let _ = writeln!(
            out,
            "{} {}",
            " ".repeat(gutter_width + 1),
            self.style.blue("|")
        );

        // Line 4: NN | source line
        let source_line = self.extract_line(line_num);
        let _ = writeln!(
            out,
            "{} {} {}",
            self.style
                .blue(&format!("{:>width$}", line_num, width = gutter_width)),
            self.style.blue("|"),
            source_line
        );

        // Line 5:   |   ^^^^ hint
        let caret_col = (span.col as usize).saturating_sub(1);
        let caret_len = if span.is_empty() {
            1
        } else {
            (span.len() as usize)
                .min(source_line.len().saturating_sub(caret_col))
                .max(1)
        };
        let caret = if caret_len == 1 {
            "^".to_string()
        } else {
            "~".repeat(caret_len)
        };
        let _ = writeln!(
            out,
            "{} {} {}{}",
            " ".repeat(gutter_width + 1),
            self.style.blue("|"),
            " ".repeat(caret_col),
            self.style.red(&format!("{} {}", caret, hint))
        );

        out
    }

    /// Render all errors, separated by blank lines.
    pub fn render_all(&self, errors: &[LexError]) -> String {
        let mut out = String::new();
        for (i, error) in errors.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&self.render(error));
        }
        out
    }
}

/// Count the number of decimal digits in a number.
fn digit_count(n: u32) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut val = n;
    while val > 0 {
        count += 1;
        val /= 10;
    }
    count
}

/// Minimal ANSI style helper. Wraps text in escape codes when color is enabled.
struct Style {
    color: bool,
}

impl Style {
    /// Red bold text (for "error[GXnnnn]").
    fn red_bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1;31m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    /// Bold text (for the error message after the colon).
    fn bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    /// Blue text (for line numbers, pipes, and arrows).
    fn blue(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[34m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    /// Red text (for carets and hints).
    fn red(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[31m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;
    use std::path::PathBuf;

    fn make_renderer<'a>(source: &'a str, file_table: &'a FileTable) -> DiagnosticRenderer<'a> {
        DiagnosticRenderer::new(source, file_table, false)
    }

    #[test]
    fn extract_line_single_line() {
        let ft = FileTable::new();
        let r = make_renderer("hello world", &ft);
        assert_eq!(r.extract_line(1), "hello world");
    }

    #[test]
    fn extract_line_multiline() {
        let ft = FileTable::new();
        let r = make_renderer("aaa\nbbb\nccc", &ft);
        assert_eq!(r.extract_line(1), "aaa");
        assert_eq!(r.extract_line(2), "bbb");
        assert_eq!(r.extract_line(3), "ccc");
    }

    #[test]
    fn extract_line_out_of_bounds() {
        let ft = FileTable::new();
        let r = make_renderer("one line", &ft);
        assert_eq!(r.extract_line(99), "");
    }

    #[test]
    fn extract_line_windows_crlf() {
        let ft = FileTable::new();
        let r = make_renderer("aaa\r\nbbb\r\n", &ft);
        assert_eq!(r.extract_line(1), "aaa");
        assert_eq!(r.extract_line(2), "bbb");
    }

    #[test]
    fn digit_count_values() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(99), 2);
        assert_eq!(digit_count(100), 3);
        assert_eq!(digit_count(999), 3);
    }

    #[test]
    fn render_contains_all_parts() {
        let mut ft = FileTable::new();
        ft.add_file(PathBuf::from("test.gx"));

        let source = "let name = \"hello\nlet y = 1";
        let r = DiagnosticRenderer::new(source, &ft, false);

        let err = LexError::UnterminatedString {
            span: Span::new(0, 12, 18, 1, 13),
        };
        let output = r.render(&err);

        assert!(output.contains("error[GX0001]"), "should have error code");
        assert!(
            output.contains("unterminated string literal"),
            "should have message"
        );
        assert!(
            output.contains("--> test.gx:1:13"),
            "should have file location"
        );
        assert!(
            output.contains("let name = \"hello"),
            "should have source line"
        );
        assert!(
            output.contains("string started here but never closed"),
            "should have hint"
        );
    }

    #[test]
    fn render_error_on_line_3() {
        let mut ft = FileTable::new();
        ft.add_file(PathBuf::from("app.gx"));

        let source = "let a = 1\nlet b = 2\nlet c = ~\nlet d = 4";
        let r = DiagnosticRenderer::new(source, &ft, false);

        let err = LexError::UnexpectedCharacter {
            span: Span::new(0, 28, 29, 3, 9),
            ch: '~',
        };
        let output = r.render(&err);

        assert!(output.contains("GX0006"));
        assert!(output.contains("3 |"));
        assert!(output.contains("let c = ~"));
    }
}
