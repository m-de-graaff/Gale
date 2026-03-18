//! Low-level character iteration with position tracking.
//!
//! The [`Cursor`] provides peek/advance operations over UTF-8 source text
//! while maintaining byte offset, line, and column counters.

/// A cursor over source text that tracks position (byte offset, line, column).
pub(crate) struct Cursor<'src> {
    source: &'src str,
    /// Remaining bytes as a slice starting from current position.
    remaining: &'src str,
    /// Current byte offset into the source.
    pos: usize,
    /// Current line number (1-based).
    line: u32,
    /// Current column (1-based, byte offset from start of line).
    col: u32,
}

impl<'src> Cursor<'src> {
    /// Create a new cursor at the start of the source.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            remaining: source,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Total length of the source in bytes.
    pub fn source_len(&self) -> usize {
        self.source.len()
    }

    /// Current byte offset in the source.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Current line number (1-based).
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Current column (1-based).
    pub fn col(&self) -> u32 {
        self.col
    }

    /// Whether we've reached the end of the source.
    #[allow(dead_code)]
    pub fn is_eof(&self) -> bool {
        self.remaining.is_empty()
    }

    /// Rewind the cursor to a previous byte offset.
    ///
    /// Recomputes line/column by scanning from the start of the source.
    /// Used by the parser when switching lexer modes requires re-lexing
    /// already-peeked tokens.
    pub fn rewind_to(&mut self, byte_offset: usize) {
        let offset = byte_offset.min(self.source.len());
        self.remaining = &self.source[offset..];
        self.pos = offset;
        // Recompute line/col by counting newlines before the offset
        self.line = 1;
        self.col = 1;
        for ch in self.source[..offset].chars() {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    /// Peek at the next character without consuming it.
    pub fn peek(&self) -> Option<char> {
        self.remaining.chars().next()
    }

    /// Peek at the second character ahead (1 past current).
    pub fn peek_second(&self) -> Option<char> {
        let mut chars = self.remaining.chars();
        chars.next();
        chars.next()
    }

    /// Peek at the third character ahead (2 past current).
    #[allow(dead_code)]
    pub fn peek_third(&self) -> Option<char> {
        let mut chars = self.remaining.chars();
        chars.next();
        chars.next();
        chars.next()
    }

    /// Consume and return the next character, updating position tracking.
    pub fn advance(&mut self) -> Option<char> {
        let ch = self.remaining.chars().next()?;
        let len = ch.len_utf8();
        self.remaining = &self.remaining[len..];
        self.pos += len;

        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += len as u32;
        }

        Some(ch)
    }

    /// Consume the next character if it matches the predicate.
    /// Returns `true` if consumed.
    pub fn eat_if(&mut self, predicate: impl FnOnce(char) -> bool) -> bool {
        match self.peek() {
            Some(ch) if predicate(ch) => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    /// Consume characters while the predicate holds.
    /// Returns the consumed slice.
    pub fn eat_while(&mut self, predicate: impl Fn(char) -> bool) -> &'src str {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if !predicate(ch) {
                break;
            }
            self.advance();
        }
        &self.source[start..self.pos]
    }

    /// Consume the next character if it equals `expected`.
    pub fn eat_char(&mut self, expected: char) -> bool {
        self.eat_if(|ch| ch == expected)
    }

    /// Get a slice of the source from `start` to `end` byte offsets.
    pub fn slice(&self, start: usize, end: usize) -> &'src str {
        &self.source[start..end]
    }

    /// Get a slice from `start` to the current position.
    pub fn slice_from(&self, start: usize) -> &'src str {
        &self.source[start..self.pos]
    }

    /// Skip whitespace (spaces and tabs only — newlines are significant).
    /// Returns `true` if any whitespace was skipped.
    pub fn skip_whitespace(&mut self) -> bool {
        let start = self.pos;
        self.eat_while(|ch| ch == ' ' || ch == '\t' || ch == '\r');
        self.pos > start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cursor_starts_at_beginning() {
        let c = Cursor::new("hello");
        assert_eq!(c.pos(), 0);
        assert_eq!(c.line(), 1);
        assert_eq!(c.col(), 1);
        assert!(!c.is_eof());
    }

    #[test]
    fn peek_does_not_advance() {
        let c = Cursor::new("abc");
        assert_eq!(c.peek(), Some('a'));
        assert_eq!(c.peek(), Some('a'));
        assert_eq!(c.pos(), 0);
    }

    #[test]
    fn advance_moves_position() {
        let mut c = Cursor::new("abc");
        assert_eq!(c.advance(), Some('a'));
        assert_eq!(c.pos(), 1);
        assert_eq!(c.col(), 2);
        assert_eq!(c.advance(), Some('b'));
        assert_eq!(c.pos(), 2);
        assert_eq!(c.col(), 3);
    }

    #[test]
    fn newline_updates_line_and_col() {
        let mut c = Cursor::new("a\nb");
        c.advance(); // 'a'
        assert_eq!(c.line(), 1);
        assert_eq!(c.col(), 2);
        c.advance(); // '\n'
        assert_eq!(c.line(), 2);
        assert_eq!(c.col(), 1);
        c.advance(); // 'b'
        assert_eq!(c.line(), 2);
        assert_eq!(c.col(), 2);
    }

    #[test]
    fn peek_second_and_third() {
        let c = Cursor::new("abc");
        assert_eq!(c.peek(), Some('a'));
        assert_eq!(c.peek_second(), Some('b'));
        assert_eq!(c.peek_third(), Some('c'));
    }

    #[test]
    fn eat_while_returns_consumed_slice() {
        let mut c = Cursor::new("aaabbb");
        let s = c.eat_while(|ch| ch == 'a');
        assert_eq!(s, "aaa");
        assert_eq!(c.pos(), 3);
        assert_eq!(c.peek(), Some('b'));
    }

    #[test]
    fn eat_if_consumes_matching() {
        let mut c = Cursor::new("ab");
        assert!(c.eat_if(|ch| ch == 'a'));
        assert!(!c.eat_if(|ch| ch == 'a'));
        assert_eq!(c.peek(), Some('b'));
    }

    #[test]
    fn eat_char_matches_exactly() {
        let mut c = Cursor::new("=>");
        assert!(c.eat_char('='));
        assert!(c.eat_char('>'));
        assert!(c.is_eof());
    }

    #[test]
    fn skip_whitespace_skips_spaces_and_tabs() {
        let mut c = Cursor::new("  \t  hello");
        assert!(c.skip_whitespace());
        assert_eq!(c.peek(), Some('h'));
        assert_eq!(c.pos(), 5);
    }

    #[test]
    fn skip_whitespace_stops_at_newline() {
        let mut c = Cursor::new("  \nhi");
        c.skip_whitespace();
        assert_eq!(c.peek(), Some('\n'));
    }

    #[test]
    fn eof_after_full_consumption() {
        let mut c = Cursor::new("ab");
        c.advance();
        c.advance();
        assert!(c.is_eof());
        assert_eq!(c.peek(), None);
        assert_eq!(c.advance(), None);
    }

    #[test]
    fn slice_from_works() {
        let mut c = Cursor::new("hello world");
        c.advance(); // h
        c.advance(); // e
        c.advance(); // l
        c.advance(); // l
        c.advance(); // o
        assert_eq!(c.slice_from(0), "hello");
        assert_eq!(c.slice(1, 4), "ell");
    }

    #[test]
    fn multibyte_utf8_tracking() {
        let mut c = Cursor::new("aé"); // é is 2 bytes in UTF-8
        c.advance(); // 'a' — 1 byte
        assert_eq!(c.pos(), 1);
        assert_eq!(c.col(), 2);
        c.advance(); // 'é' — 2 bytes
        assert_eq!(c.pos(), 3);
        assert_eq!(c.col(), 4); // column tracks bytes
        assert!(c.is_eof());
    }
}
