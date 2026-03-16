//! Code-mode lexing: identifiers, keywords, operators, delimiters, comments.

use unicode_xid::UnicodeXID;

use super::Lexer;
use crate::error::LexError;
use crate::token::{lookup_keyword, Token, TokenWithSpan};

impl<'src> Lexer<'src> {
    /// Main code-mode lexer entry point.
    ///
    /// Handles whitespace, newlines, comments, identifiers/keywords,
    /// operators, delimiters, and delegates to specialized sub-lexers
    /// for numbers, strings, template literals, and regex.
    pub(super) fn lex_code(&mut self) -> TokenWithSpan {
        // Skip horizontal whitespace (spaces, tabs, \r)
        self.cursor.skip_whitespace();

        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let ch = match self.cursor.peek() {
            Some(ch) => ch,
            None => {
                let span = self.span_from(start, line, col);
                return (Token::EOF, span);
            }
        };

        match ch {
            // ── Newline ───────────────────────────────────────────
            '\n' => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                (Token::Newline, span)
            }

            // ── Comments ──────────────────────────────────────────
            '/' if self.cursor.peek_second() == Some('/') => self.lex_line_comment(),
            '/' if self.cursor.peek_second() == Some('*') => self.lex_block_comment(),

            // ── Identifiers & keywords ────────────────────────────
            ch if UnicodeXID::is_xid_start(ch) || ch == '_' => self.lex_identifier(),

            // ── Number literals ───────────────────────────────────
            '0'..='9' => self.lex_number(),

            // ── String literals ───────────────────────────────────
            '"' => self.lex_string(),

            // ── Template literals ─────────────────────────────────
            '`' => self.lex_template_literal(),

            // ── Regex or slash ────────────────────────────────────
            '/' => self.lex_slash_or_regex(),

            // ── Operators & delimiters ────────────────────────────
            _ => self.lex_operator_or_delimiter(),
        }
    }

    /// Lex an identifier or keyword.
    fn lex_identifier(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let ident = self
            .cursor
            .eat_while(|ch| UnicodeXID::is_xid_continue(ch) || ch == '_');
        let ident_owned = ident.to_string();
        let span = self.span_from(start, line, col);

        // Check if it's a keyword
        match lookup_keyword(&ident_owned) {
            Some(kw_token) => (kw_token, span),
            None => (Token::Ident(ident_owned), span),
        }
    }

    /// Lex a line comment: `// ...`
    pub(super) fn lex_line_comment(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        // Consume `//`
        self.cursor.advance();
        self.cursor.advance();

        // Consume until newline (but don't consume the newline)
        let text_start = self.cursor.pos();
        self.cursor.eat_while(|ch| ch != '\n');
        let text = self.cursor.slice(text_start, self.cursor.pos()).to_string();

        let span = self.span_from(start, line, col);
        (Token::Comment(text.trim_start().to_string()), span)
    }

    /// Lex a block comment: `/* ... */` (nestable).
    pub(super) fn lex_block_comment(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        // Consume `/*`
        self.cursor.advance();
        self.cursor.advance();

        let text_start = self.cursor.pos();
        let mut depth: u32 = 1;

        while depth > 0 {
            match self.cursor.peek() {
                None => {
                    // Unterminated block comment
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedBlockComment { span });
                    let text = self.cursor.slice(text_start, self.cursor.pos()).to_string();
                    return (Token::BlockComment(text), span);
                }
                Some('/') if self.cursor.peek_second() == Some('*') => {
                    self.cursor.advance();
                    self.cursor.advance();
                    depth += 1;
                }
                Some('*') if self.cursor.peek_second() == Some('/') => {
                    depth -= 1;
                    if depth == 0 {
                        let text = self.cursor.slice(text_start, self.cursor.pos()).to_string();
                        self.cursor.advance(); // consume `*`
                        self.cursor.advance(); // consume `/`
                        let span = self.span_from(start, line, col);
                        return (Token::BlockComment(text.trim().to_string()), span);
                    }
                    self.cursor.advance();
                    self.cursor.advance();
                }
                _ => {
                    self.cursor.advance();
                }
            }
        }

        unreachable!()
    }

    /// Lex an operator or delimiter character.
    pub(super) fn lex_operator_or_delimiter(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();
        let ch = self.cursor.advance().unwrap();

        let token = match ch {
            // ── Multi-char operators (check longest match first) ──

            // + +=
            '+' => {
                if self.cursor.eat_char('=') {
                    Token::PlusEq
                } else {
                    Token::Plus
                }
            }

            // - -= ->
            '-' => {
                if self.cursor.eat_char('=') {
                    Token::MinusEq
                } else if self.cursor.eat_char('>') {
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }

            // *
            '*' => Token::Star,

            // %
            '%' => Token::Percent,

            // = == =>
            '=' => {
                if self.cursor.eat_char('=') {
                    Token::EqEq
                } else if self.cursor.eat_char('>') {
                    Token::FatArrow
                } else {
                    Token::Eq
                }
            }

            // ! !=
            '!' => {
                if self.cursor.eat_char('=') {
                    Token::NotEq
                } else {
                    Token::Not
                }
            }

            // < <= <-> <  (LAngle in template context handled elsewhere)
            '<' => {
                if self.cursor.eat_char('=') {
                    Token::LessEq
                } else if self.cursor.peek() == Some('-') && self.cursor.peek_second() == Some('>')
                {
                    self.cursor.advance(); // -
                    self.cursor.advance(); // >
                    Token::BiArrow
                } else {
                    Token::Less
                }
            }

            // > >=
            '>' => {
                if self.cursor.eat_char('=') {
                    Token::GreaterEq
                } else {
                    Token::Greater
                }
            }

            // & &&
            '&' => {
                if self.cursor.eat_char('&') {
                    Token::And
                } else {
                    // Bare `&` — not defined in GaleX, treat as unexpected
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnexpectedCharacter { span, ch: '&' });
                    Token::Ident("&".into())
                }
            }

            // | || |>  and bare |
            '|' => {
                if self.cursor.eat_char('|') {
                    Token::Or
                } else if self.cursor.eat_char('>') {
                    Token::Pipe
                } else {
                    Token::Bar
                }
            }

            // ? ?. ??
            '?' => {
                if self.cursor.eat_char('.') {
                    Token::QuestionDot
                } else if self.cursor.eat_char('?') {
                    Token::NullCoalesce
                } else {
                    Token::Question
                }
            }

            // . .. ...
            '.' => {
                if self.cursor.peek() == Some('.') && self.cursor.peek_second() == Some('.') {
                    self.cursor.advance();
                    self.cursor.advance();
                    Token::Spread
                } else if self.cursor.eat_char('.') {
                    Token::DotDot
                } else {
                    Token::Dot
                }
            }

            // ── Simple delimiters ─────────────────────────────────
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            ',' => Token::Comma,
            '@' => Token::At,
            '#' => Token::Hash,

            // ── Unexpected character ──────────────────────────────
            _ => {
                let span = self.span_from(start, line, col);
                self.emit_error(LexError::UnexpectedCharacter { span, ch });
                // Skip and try next token
                return self.lex_code();
            }
        };

        let span = self.span_from(start, line, col);
        (token, span)
    }

    /// Lex `/` — could be slash (division), or regex literal.
    fn lex_slash_or_regex(&mut self) -> TokenWithSpan {
        // Check if previous token can end an expression → division
        if self.prev_can_end_expr {
            let start = self.cursor.pos();
            let line = self.cursor.line();
            let col = self.cursor.col();
            self.cursor.advance();
            let span = self.span_from(start, line, col);
            (Token::Slash, span)
        } else {
            self.lex_regex()
        }
    }
}
