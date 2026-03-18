//! Template-mode lexing: HTML tags, directives, text nodes.
//!
//! Template mode is active inside component bodies. It handles:
//! - HTML open tags: `<div`, `<Button`
//! - HTML close tags: `</div>`
//! - Self-closing: `/>`
//! - Quoted text nodes: `"text"`
//! - Expression interpolation: `{expr}`
//! - Template control flow keywords: `when`, `each`, `suspend`, `slot`
//!
//! HtmlTag mode is active inside `<tag ...>` and handles:
//! - Attributes: `class="value"`, `disabled`
//! - Directives: `bind:x`, `on:click.prevent`, `class:name`, etc.
//! - Tag termination: `>` or `/>`

use unicode_xid::UnicodeXID;

use super::{LexMode, Lexer};
use crate::error::LexError;
use crate::token::{lookup_keyword, Token, TokenWithSpan};

impl<'src> Lexer<'src> {
    /// Lex in Template mode (inside a component body).
    pub(super) fn lex_template(&mut self) -> TokenWithSpan {
        // Skip whitespace (including newlines in template mode — they're not significant)
        self.skip_template_whitespace();

        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        match self.cursor.peek() {
            None => {
                let span = self.span_from(start, line, col);
                (Token::EOF, span)
            }

            // HTML close tag: </tagname>
            Some('<') if self.cursor.peek_second() == Some('/') => self.lex_html_close_tag(),

            // HTML open tag: <tagname
            Some('<')
                if self
                    .cursor
                    .peek_second()
                    .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_') =>
            {
                self.lex_html_open_tag()
            }

            // Expression interpolation
            Some('{') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                self.push_mode(LexMode::TemplateExpr { depth: 0 });
                (Token::ExprOpen, span)
            }

            // Closing brace (end of template block like `when ... { }`)
            Some('}') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                self.pop_mode(); // pop Template mode
                (Token::RBrace, span)
            }

            // Quoted text node
            Some('"') => self.lex_html_text(),

            // Template literal
            Some('`') => self.lex_template_literal(),

            // Identifier or keyword (when, each, suspend, slot, derive, let, etc.)
            Some(ch) if UnicodeXID::is_xid_start(ch) || ch == '_' => self.lex_template_identifier(),

            // Comments in template mode
            Some('/') if self.cursor.peek_second() == Some('/') => self.lex_line_comment(),
            Some('/') if self.cursor.peek_second() == Some('*') => self.lex_block_comment(),

            // Self-close (shouldn't normally appear here, but handle gracefully)
            Some('/') if self.cursor.peek_second() == Some('>') => {
                self.cursor.advance();
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                (Token::HtmlSelfClose, span)
            }

            // Unquoted text content — everything else until we hit a special character.
            // This allows natural HTML like `<h1>Hello world</h1>` without quotes.
            _ => self.lex_bare_text(),
        }
    }

    /// Lex unquoted text content inside a template.
    ///
    /// Accumulates all characters until hitting `<`, `{`, `}`, or EOF.
    /// Produces an `HtmlText` token. Leading/trailing whitespace within
    /// the text is preserved (template whitespace was already skipped).
    fn lex_bare_text(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let mut text = String::new();
        loop {
            match self.cursor.peek() {
                // Stop at HTML tags, expressions, block boundaries, or EOF
                None | Some('<') | Some('{') | Some('}') => break,
                Some(ch) => {
                    self.cursor.advance();
                    text.push(ch);
                }
            }
        }

        // Trim trailing whitespace (leading was already skipped)
        let trimmed = text.trim_end().to_string();
        let span = self.span_from(start, line, col);
        (Token::HtmlText(trimmed), span)
    }

    /// Lex an HTML open tag: `<tagname`
    fn lex_html_open_tag(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume `<`

        // Read tag name
        let name = self
            .cursor
            .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
            .to_string();

        let span = self.span_from(start, line, col);
        self.push_mode(LexMode::HtmlTag);
        (Token::HtmlOpen(name), span)
    }

    /// Lex an HTML close tag: `</tagname>`
    fn lex_html_close_tag(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume `<`
        self.cursor.advance(); // consume `/`

        // Read tag name
        let name = self
            .cursor
            .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
            .to_string();

        // Skip whitespace and consume `>`
        self.cursor.skip_whitespace();
        self.cursor.eat_char('>');

        let span = self.span_from(start, line, col);
        (Token::HtmlClose(name), span)
    }

    /// Lex a quoted text node in template: `"text content"`
    fn lex_html_text(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        self.cursor.advance(); // consume opening `"`

        let mut value = String::new();

        loop {
            match self.cursor.peek() {
                None | Some('\n') => {
                    let span = self.span_from(start, line, col);
                    self.emit_error(LexError::UnterminatedString { span });
                    return (Token::HtmlText(value), span);
                }
                Some('"') => {
                    self.cursor.advance(); // consume closing `"`
                    let span = self.span_from(start, line, col);
                    return (Token::HtmlText(value), span);
                }
                Some('\\') => {
                    self.cursor.advance();
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

    /// Lex an identifier in template mode.
    ///
    /// Only the template-control keywords (`when`, `each`, `suspend`, `slot`)
    /// are recognised as keywords.  Everything else is treated as bare text
    /// content so that `<p>Hello world</p>` works without quotes.
    fn lex_template_identifier(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        let ident = self
            .cursor
            .eat_while(|ch| UnicodeXID::is_xid_continue(ch) || ch == '_');
        let ident_owned = ident.to_string();

        // Only recognise template-control keywords in template mode.
        if matches!(ident_owned.as_str(), "when" | "each" | "suspend" | "slot") {
            let span = self.span_from(start, line, col);
            return (lookup_keyword(&ident_owned).unwrap(), span);
        }

        // Not a template keyword — treat as bare text content.
        // Continue scanning until we hit a tag, expression, or block boundary.
        let mut text = ident_owned;
        loop {
            match self.cursor.peek() {
                None | Some('<') | Some('{') | Some('}') => break,
                Some(ch) => {
                    self.cursor.advance();
                    text.push(ch);
                }
            }
        }
        let trimmed = text.trim_end().to_string();
        let span = self.span_from(start, line, col);
        (Token::HtmlText(trimmed), span)
    }

    /// Lex inside an HTML tag: attributes, directives, `>`, `/>`.
    pub(super) fn lex_html_tag(&mut self) -> TokenWithSpan {
        // Skip whitespace (including newlines inside tags)
        self.skip_template_whitespace();

        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        match self.cursor.peek() {
            None => {
                let span = self.span_from(start, line, col);
                self.pop_mode();
                (Token::EOF, span)
            }

            // Self-closing tag: />
            Some('/') if self.cursor.peek_second() == Some('>') => {
                self.cursor.advance(); // /
                self.cursor.advance(); // >
                let span = self.span_from(start, line, col);
                self.pop_mode(); // pop HtmlTag
                (Token::HtmlSelfClose, span)
            }

            // Tag close: >
            Some('>') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                self.pop_mode(); // pop HtmlTag
                (Token::RAngle, span)
            }

            // Attribute value: "string"
            Some('"') => self.lex_string(),

            // Template literal attribute value
            Some('`') => self.lex_template_literal(),

            // Expression attribute value: {expr}
            Some('{') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                self.push_mode(LexMode::TemplateExpr { depth: 0 });
                (Token::ExprOpen, span)
            }

            // = (attribute assignment)
            Some('=') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                (Token::Eq, span)
            }

            // Attribute name, directive, or boolean attribute
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                self.lex_html_attribute_or_directive()
            }

            // Colon (standalone, e.g., after on:click for modifiers — shouldn't normally hit this)
            Some(':') => {
                self.cursor.advance();
                let span = self.span_from(start, line, col);
                (Token::Colon, span)
            }

            // Anything else — error and skip
            _ => {
                let ch = self.cursor.advance().unwrap();
                let span = self.span_from(start, line, col);
                self.emit_error(LexError::UnexpectedCharacter { span, ch });
                self.lex_html_tag() // try again
            }
        }
    }

    /// Lex an attribute name or directive inside an HTML tag.
    ///
    /// Recognizes directive prefixes and lexes them as compound tokens:
    /// - `bind:x` → `BindDir("x")`
    /// - `on:click.prevent` → `OnDir { event: "click", modifiers: ["prevent"] }`
    /// - `class:name` → `ClassDir("name")`
    /// - `ref:name` → `RefDir("name")`
    /// - `transition:type` → `TransDir("type")`
    /// - `into:slot` → `IntoDir("slot")`
    /// - `form:action` → `FormAction`
    /// - `form:guard` → `FormGuard`
    /// - `form:error` → `FormError`
    /// - `key` → `KeyDir`
    /// - `prefetch` → `Prefetch`
    fn lex_html_attribute_or_directive(&mut self) -> TokenWithSpan {
        let start = self.cursor.pos();
        let line = self.cursor.line();
        let col = self.cursor.col();

        // Read the first identifier segment
        let prefix = self
            .cursor
            .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            .to_string();

        // Check for standalone directive keywords
        if self.cursor.peek() != Some(':') {
            let span = self.span_from(start, line, col);
            return match prefix.as_str() {
                "key" => (Token::KeyDir, span),
                "prefetch" => (Token::Prefetch, span),
                _ => (Token::Ident(prefix), span),
            };
        }

        // Has a colon — check directive prefixes
        match prefix.as_str() {
            "bind" => {
                self.cursor.advance(); // consume `:`
                let target = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    .to_string();
                let span = self.span_from(start, line, col);
                (Token::BindDir(target), span)
            }
            "on" => {
                self.cursor.advance(); // consume `:`
                let event = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    .to_string();
                // Parse modifiers: .prevent.once.self
                let mut modifiers = Vec::new();
                while self.cursor.eat_char('.') {
                    let modifier = self
                        .cursor
                        .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                        .to_string();
                    if !modifier.is_empty() {
                        modifiers.push(modifier);
                    }
                }
                let span = self.span_from(start, line, col);
                (Token::OnDir { event, modifiers }, span)
            }
            "class" => {
                self.cursor.advance(); // consume `:`
                let name = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    .to_string();
                let span = self.span_from(start, line, col);
                (Token::ClassDir(name), span)
            }
            "ref" => {
                self.cursor.advance(); // consume `:`
                let name = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    .to_string();
                let span = self.span_from(start, line, col);
                (Token::RefDir(name), span)
            }
            "transition" => {
                self.cursor.advance(); // consume `:`
                let name = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    .to_string();
                let span = self.span_from(start, line, col);
                (Token::TransDir(name), span)
            }
            "into" => {
                self.cursor.advance(); // consume `:`
                let slot = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    .to_string();
                let span = self.span_from(start, line, col);
                (Token::IntoDir(slot), span)
            }
            "form" => {
                self.cursor.advance(); // consume `:`
                let kind = self
                    .cursor
                    .eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    .to_string();
                let span = self.span_from(start, line, col);
                match kind.as_str() {
                    "action" => (Token::FormAction, span),
                    "guard" => (Token::FormGuard, span),
                    "error" => (Token::FormError, span),
                    _ => {
                        // Unknown form directive — treat as ident
                        (Token::Ident(format!("form:{}", kind)), span)
                    }
                }
            }
            _ => {
                // Not a recognized directive prefix, but has a colon.
                // This could be a namespaced attribute like `xml:lang`.
                // Don't consume the colon — return the prefix as Ident.
                let span = self.span_from(start, line, col);
                (Token::Ident(prefix), span)
            }
        }
    }

    /// Skip whitespace including newlines (for template/tag modes).
    fn skip_template_whitespace(&mut self) {
        self.cursor
            .eat_while(|ch| ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n');
    }
}
