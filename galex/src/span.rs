//! Source location tracking for tokens and error reporting.

use std::fmt;
use std::path::{Path, PathBuf};

/// A region in source code, identified by byte offsets and line/column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Index into the shared [`FileTable`].
    pub file_id: u32,
    /// Start byte offset in the source (inclusive).
    pub start: u32,
    /// End byte offset in the source (exclusive).
    pub end: u32,
    /// 1-based line number where the span starts.
    pub line: u32,
    /// 1-based column (byte offset from start of line) where the span starts.
    pub col: u32,
}

impl Span {
    /// Create a new span.
    pub fn new(file_id: u32, start: u32, end: u32, line: u32, col: u32) -> Self {
        Self {
            file_id,
            start,
            end,
            line,
            col,
        }
    }

    /// Create a dummy span for testing or synthetic tokens.
    pub fn dummy() -> Self {
        Self {
            file_id: 0,
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        }
    }

    /// Length of the span in bytes.
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Whether this span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one that covers both.
    /// Check if a byte offset falls within this span.
    pub fn contains_offset(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Compute the end (line, col) given the source text.
    ///
    /// The `Span` only stores start line/col. This method scans the source
    /// text to find the end position — needed for LSP `Range` conversion.
    pub fn end_position(&self, source: &str) -> (u32, u32) {
        let mut line = 1u32;
        let mut col = 1u32;
        for (i, ch) in source.char_indices() {
            if i as u32 >= self.end {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(self.file_id, other.file_id);
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        // Use the position of whichever span starts first
        let (line, col) = if self.start <= other.start {
            (self.line, self.col)
        } else {
            (other.line, other.col)
        };
        Span {
            file_id: self.file_id,
            start,
            end,
            line,
            col,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Shared file table mapping file IDs to paths.
///
/// Avoids cloning `PathBuf` into every span — tokens just store a `u32` file ID.
#[derive(Debug, Default)]
pub struct FileTable {
    files: Vec<PathBuf>,
}

impl FileTable {
    /// Create an empty file table.
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Register a file and return its ID.
    pub fn add_file(&mut self, path: PathBuf) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(path);
        id
    }

    /// Get the path for a file ID.
    pub fn get_path(&self, file_id: u32) -> Option<&Path> {
        self.files.get(file_id as usize).map(|p| p.as_path())
    }

    /// Find an existing file ID by path, or register a new one.
    ///
    /// Unlike [`add_file`], this avoids duplicate entries for the same path.
    pub fn find_or_add_file(&mut self, path: PathBuf) -> u32 {
        for (id, existing) in self.files.iter().enumerate() {
            if existing == &path {
                return id as u32;
            }
        }
        self.add_file(path)
    }

    /// Format a span with its file path for error messages.
    pub fn format_span(&self, span: &Span) -> String {
        match self.get_path(span.file_id) {
            Some(path) => format!("{}:{}:{}", path.display(), span.line, span.col),
            None => format!("<unknown>:{}:{}", span.line, span.col),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_len_and_empty() {
        let s = Span::new(0, 10, 15, 1, 11);
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());

        let empty = Span::new(0, 5, 5, 1, 6);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn span_merge() {
        let a = Span::new(0, 5, 10, 1, 6);
        let b = Span::new(0, 12, 20, 1, 13);
        let merged = a.merge(b);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 20);
        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 6);
    }

    #[test]
    fn span_display() {
        let s = Span::new(0, 0, 5, 3, 7);
        assert_eq!(format!("{}", s), "3:7");
    }

    #[test]
    fn file_table_round_trip() {
        let mut ft = FileTable::new();
        let id = ft.add_file(PathBuf::from("test.gx"));
        assert_eq!(id, 0);
        assert_eq!(ft.get_path(id), Some(Path::new("test.gx")));
        assert_eq!(ft.get_path(99), None);
    }

    #[test]
    fn file_table_format_span() {
        let mut ft = FileTable::new();
        ft.add_file(PathBuf::from("src/app.gx"));
        let span = Span::new(0, 0, 5, 10, 3);
        assert_eq!(ft.format_span(&span), "src/app.gx:10:3");
    }
}
