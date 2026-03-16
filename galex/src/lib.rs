//! GaleX language lexer and compiler toolchain.
//!
//! This crate provides the lexer (tokenizer) for the GaleX language,
//! converting `.gx` source files into a stream of tokens for parsing.
//!
//! # Quick start
//!
//! ```
//! use galex::lex;
//!
//! let result = galex::lex("let x = 42", 0);
//! assert!(result.is_ok());
//! for (token, span) in &result.tokens {
//!     println!("{:?} at {}:{}", token, span.line, span.col);
//! }
//! ```

pub mod diagnostic;
pub mod error;
pub mod lexer;
pub mod span;
pub mod token;

// Re-export key types for convenience
pub use diagnostic::DiagnosticRenderer;
pub use error::{LexError, LexResult};
pub use lexer::{lex, LexMode, Lexer};
pub use span::{FileTable, Span};
pub use token::{Token, TokenWithSpan};
