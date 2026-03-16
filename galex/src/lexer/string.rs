//! String and template literal lexing.
//!
//! - Double-quoted strings: `"hello\n"` with escape sequences
//! - Template literals: `` `text ${expr} more` `` with segmented tokens
//!
//! Escape sequences: `\\`, `\"`, `\n`, `\r`, `\t`, `\0`, `\x{HH}`, `\u{HHHH}`

use super::{LexMode, Lexer};
use crate::error::LexError;
use crate::token::{Token, TokenWithSpan};

impl<'src> Lexer<'src> {
    /// Lex a double-quoted string literal.
    pub(super) fn lex_string(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume opening `"`

        let mut value = String::new();

        loop {
            match self.cursor.peek() {
                None | Some('\n') => {
                    // Unterminated string
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedString { span });
                    return (Token::StringLit(value), span);
                }
                Some('"') => {
                    self.cursor.advance(); // consume closing `"`
                    let span = self.span_from(start, line, col);
                    return (Token::StringLit(value), span);
                }
                Some('\\') => {
                    self.cursor.advance(); // consume `\`
                    match self.lex_escape_sequence() {
                        Ok(ch) => value.push(ch),
                        Err(bad_ch) => {
                            let span = self.span_from(start, line, col);
                            self.emit_error(LexError::InvalidEscapeSequence {
                                span,
                                sequence: bad_ch,
                            });
                            value.push(bad_ch);
                        }
                    }
                }
                Some(ch) => {
                    self.cursor.advance();
                    value.push(ch);
                }
            }
        }
    }

    /// Lex a template literal starting with `` ` ``.
    ///
    /// Produces either:
    /// - `TemplateNoSub(text)` for `` `text` `` with no interpolation
    /// - `TemplateHead(text)` for `` `text${ `` then pushes TemplateExpr mode
    pub(super) fn lex_template_literal(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume opening `` ` ``
        self.push_mode(LexMode::TemplateLiteral);

        let mut value = String::new();

        loop {
            match self.cursor.peek() {
                None => {
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedTemplateLiteral { span });
                    self.pop_mode(); // pop TemplateLiteral
                    return (Token::TemplateNoSub(value), span);
                }
                Some('`') => {
                    self.cursor.advance(); // consume closing `` ` ``
                    let span = self.span_from(start, line, col);
                    self.pop_mode(); // pop TemplateLiteral
                    return (Token::TemplateNoSub(value), span);
                }
                Some('$') if self.cursor.peek_second() == Some('{') => {
                    self.cursor.advance(); // consume `$`
                    self.cursor.advance(); // consume `{`
                    let span = self.span_from(start, line, col);
                    // Push TemplateExpr mode (the TemplateLiteral mode stays on stack)
                    self.push_mode(LexMode::TemplateExpr { depth: 0 });
                    return (Token::TemplateHead(value), span);
                }
                Some('\\') => {
                    self.cursor.advance(); // consume `\`
                    match self.lex_escape_sequence() {
                        Ok(ch) => value.push(ch),
                        Err(bad_ch) => {
                            let span = self.span_from(start, line, col);
                            self.emit_error(LexError::InvalidEscapeSequence {
                                span,
                                sequence: bad_ch,
                            });
                            value.push(bad_ch);
                        }
                    }
                }
                Some(ch) => {
                    self.cursor.advance();
                    value.push(ch);
                }
            }
        }
    }

    /// Continue lexing a template literal after `}` closes an expression.
    ///
    /// Produces either:
    /// - `TemplateTail(text)` if we hit `` ` `` (end of template literal)
    /// - `TemplateMiddle(text)` if we hit another `${`
    pub(super) fn lex_template_literal_after_expr(
        &mut self,
        _brace_start: usize,
        _brace_line: u32,
        _brace_col: u32,
    ) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let mut value = String::new();

        loop {
            match self.cursor.peek() {
                None => {
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedTemplateLiteral { span });
                    self.pop_mode(); // pop TemplateLiteral
                    return (Token::TemplateTail(value), span);
                }
                Some('`') => {
                    self.cursor.advance(); // consume closing `` ` ``
                    let span = self.span_from(start, line, col);
                    self.pop_mode(); // pop TemplateLiteral
                    return (Token::TemplateTail(value), span);
                }
                Some('$') if self.cursor.peek_second() == Some('{') => {
                    self.cursor.advance(); // consume `$`
                    self.cursor.advance(); // consume `{`
                    let span = self.span_from(start, line, col);
                    // Push TemplateExpr mode
                    self.push_mode(LexMode::TemplateExpr { depth: 0 });
                    return (Token::TemplateMiddle(value), span);
                }
                Some('\\') => {
                    self.cursor.advance(); // consume `\`
                    match self.lex_escape_sequence() {
                        Ok(ch) => value.push(ch),
                        Err(bad_ch) => {
                            let span = self.span_from(start, line, col);
                            self.emit_error(LexError::InvalidEscapeSequence {
                                span,
                                sequence: bad_ch,
                            });
                            value.push(bad_ch);
                        }
                    }
                }
                Some(ch) => {
                    self.cursor.advance();
                    value.push(ch);
                }
            }
        }
    }

    /// Called from TemplateLiteral mode's next_token to continue reading.
    /// This handles the case where we're in TemplateLiteral mode on the stack.
    pub(super) fn lex_template_literal_continuation(&mut self) -> TokenWithSpan {
        // This is called when the mode stack has TemplateLiteral on top.
        // It means we're continuing after an expression closed.
        // Use the same logic as after_expr.
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();
        self.lex_template_literal_after_expr(start, line, col)
    }

    /// Lex an escape sequence after the `\` has been consumed.
    /// Returns the character it represents, or `Err(ch)` for invalid sequences.
    pub(super) fn lex_escape_sequence(&mut self) -> Result<char, char> {
        match self.cursor.advance() {
            Some('\\') => Ok('\\'),
            Some('"') => Ok('"'),
            Some('\'') => Ok('\''),
            Some('`') => Ok('`'),
            Some('{') => Ok('{'),
            Some('$') => Ok('$'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('0') => Ok('\0'),
            Some('x') => self.lex_hex_escape(2),
            Some('u') => self.lex_unicode_escape(),
            Some(ch) => Err(ch),
            None => Err(' '), // EOF after backslash
        }
    }

    /// Lex `\xHH` — exactly 2 hex digits.
    fn lex_hex_escape(&mut self, count: usize) -> Result<char, char> {
        let mut value: u32 = 0;
        for _ in 0..count {
            match self.cursor.peek() {
                Some(ch) if ch.is_ascii_hexdigit() => {
                    self.cursor.advance();
                    value = value * 16 + ch.to_digit(16).unwrap();
                }
                _ => return Err('x'),
            }
        }
        char::from_u32(value).ok_or('x')
    }

    /// Lex `\u{HHHH}` — 1 to 6 hex digits in braces.
    fn lex_unicode_escape(&mut self) -> Result<char, char> {
        if !self.cursor.eat_char('{') {
            return Err('u');
        }

        let mut value: u32 = 0;
        let mut count = 0;

        while count < 6 {
            match self.cursor.peek() {
                Some(ch) if ch.is_ascii_hexdigit() => {
                    self.cursor.advance();
                    value = value * 16 + ch.to_digit(16).unwrap();
                    count += 1;
                }
                Some('}') => break,
                _ => return Err('u'),
            }
        }

        if count == 0 || !self.cursor.eat_char('}') {
            return Err('u');
        }

        char::from_u32(value).ok_or('u')
    }
}
