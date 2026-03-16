//! Number literal lexing: integers and floats.
//!
//! Supports:
//! - Decimal: `42`, `1_000_000`
//! - Hexadecimal: `0xFF`, `0XFF`
//! - Binary: `0b1010`, `0B1010`
//! - Float: `3.14`, `0.5` (no leading-dot like `.5`)
//! - Visual separators: `1_000_000`, `0xFF_FF`

use super::Lexer;
use crate::error::LexError;
use crate::token::{Token, TokenWithSpan};

impl<'src> Lexer<'src> {
    /// Lex a number literal starting with a digit.
    pub(super) fn lex_number(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let first = self.cursor.advance().unwrap(); // guaranteed digit

        // Check for hex/binary prefix
        if first == '0' {
            match self.cursor.peek() {
                Some('x') | Some('X') => return self.lex_hex(start, line, col),
                Some('b') | Some('B') => return self.lex_binary(start, line, col),
                _ => {}
            }
        }

        // Decimal integer or float
        self.cursor.eat_while(|ch| ch.is_ascii_digit() || ch == '_');

        // Check for float: digit followed by `.` then another digit
        // (don't match `..` range operator or `.method()` calls)
        if self.cursor.peek() == Some('.')
            && self
                .cursor
                .peek_second()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.cursor.advance(); // consume `.`
            self.cursor.eat_while(|ch| ch.is_ascii_digit() || ch == '_');
            return self.finish_float(start, line, col);
        }

        self.finish_int(start, line, col, 10)
    }

    /// Lex hexadecimal: `0x` already consumed `0`, now consume `x` and hex digits.
    fn lex_hex(&mut self, start: usize, line: u32, col: u32) -> TokenWithSpan {
        self.cursor.advance(); // consume 'x' or 'X'

        let digits_start = self.cursor.pos();
        self.cursor
            .eat_while(|ch| ch.is_ascii_hexdigit() || ch == '_');

        if self.cursor.pos() == digits_start {
            // No digits after 0x
            let span = self.span_from(start, line, col);
            self.emit_error(LexError::InvalidNumberLiteral {
                span,
                reason: "expected hex digits after `0x`".into(),
            });
            return (Token::IntLit(0), span);
        }

        self.finish_int(start, line, col, 16)
    }

    /// Lex binary: `0b` already consumed `0`, now consume `b` and binary digits.
    fn lex_binary(&mut self, start: usize, line: u32, col: u32) -> TokenWithSpan {
        self.cursor.advance(); // consume 'b' or 'B'

        let digits_start = self.cursor.pos();
        self.cursor
            .eat_while(|ch| ch == '0' || ch == '1' || ch == '_');

        if self.cursor.pos() == digits_start {
            let span = self.span_from(start, line, col);
            self.emit_error(LexError::InvalidNumberLiteral {
                span,
                reason: "expected binary digits after `0b`".into(),
            });
            return (Token::IntLit(0), span);
        }

        self.finish_int(start, line, col, 2)
    }

    /// Parse the collected digits as an integer.
    fn finish_int(&mut self, start: usize, line: u32, col: u32, radix: u32) -> TokenWithSpan {
        let raw = self.cursor.slice_from(start);
        let span = self.span_from(start, line, col);

        // Strip prefix and underscores for parsing
        let clean: String = if radix == 16 {
            raw[2..].chars().filter(|&c| c != '_').collect()
        } else if radix == 2 {
            raw[2..].chars().filter(|&c| c != '_').collect()
        } else {
            raw.chars().filter(|&c| c != '_').collect()
        };

        match i64::from_str_radix(&clean, radix) {
            Ok(value) => (Token::IntLit(value), span),
            Err(_) => {
                self.emit_error(LexError::InvalidNumberLiteral {
                    span,
                    reason: format!("integer literal too large or invalid: `{}`", raw),
                });
                (Token::IntLit(0), span)
            }
        }
    }

    /// Parse the collected digits as a float.
    fn finish_float(&mut self, start: usize, line: u32, col: u32) -> TokenWithSpan {
        let raw = self.cursor.slice_from(start);
        let span = self.span_from(start, line, col);

        let clean: String = raw.chars().filter(|&c| c != '_').collect();

        match clean.parse::<f64>() {
            Ok(value) => (Token::FloatLit(value), span),
            Err(_) => {
                self.emit_error(LexError::InvalidNumberLiteral {
                    span,
                    reason: format!("invalid float literal: `{}`", raw),
                });
                (Token::FloatLit(0.0), span)
            }
        }
    }
}
