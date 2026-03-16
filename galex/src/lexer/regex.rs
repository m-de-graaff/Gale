//! Regex literal lexing with `/pattern/flags` syntax.
//!
//! Disambiguation between `/` (division) and `/pattern/` (regex) is handled
//! by the caller — this module is only invoked when regex is expected.

use super::Lexer;
use crate::error::LexError;
use crate::token::{Token, TokenWithSpan};

impl<'src> Lexer<'src> {
    /// Lex a regex literal: `/pattern/flags`.
    ///
    /// Called when the lexer has determined `/` should start a regex
    /// (i.e., the previous token cannot end an expression).
    pub(super) fn lex_regex(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume opening `/`

        let mut pattern = String::new();
        let mut in_char_class = false;

        loop {
            match self.cursor.peek() {
                None | Some('\n') => {
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedRegex { span });
                    return (
                        Token::RegexLit {
                            pattern,
                            flags: String::new(),
                        },
                        span,
                    );
                }
                Some('/') if !in_char_class => {
                    self.cursor.advance(); // consume closing `/`
                    break;
                }
                Some('[') => {
                    in_char_class = true;
                    self.cursor.advance();
                    pattern.push('[');
                }
                Some(']') if in_char_class => {
                    in_char_class = false;
                    self.cursor.advance();
                    pattern.push(']');
                }
                Some('\\') => {
                    self.cursor.advance(); // consume `\`
                    pattern.push('\\');
                    // Consume the escaped character
                    if let Some(escaped) = self.cursor.advance() {
                        pattern.push(escaped);
                    }
                }
                Some(ch) => {
                    self.cursor.advance();
                    pattern.push(ch);
                }
            }
        }

        // Lex optional flags (e.g., `i`, `g`, `m`, `s`, `u`, `y`)
        let flags_start = self.cursor.pos();
        self.cursor.eat_while(|ch| ch.is_ascii_alphabetic());
        let flags = self
            .cursor
            .slice(flags_start, self.cursor.pos())
            .to_string();

        let span = self.span_from(start, line, col);
        (Token::RegexLit { pattern, flags }, span)
    }
}
