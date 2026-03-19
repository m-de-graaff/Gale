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

pub mod ast;
pub mod checker;
pub mod codegen;
pub mod commands;
pub mod compiler;
pub mod config;
pub mod dev;
pub mod diagnostic;
pub mod error;
pub mod errors;
pub mod fmt;
pub mod lexer;
pub mod lint;
pub mod lsp;
pub mod minify;
pub mod parser;
pub mod registry;
pub mod router;
pub mod span;
pub mod tailwind;
pub mod token;
pub mod types;

// Re-export key types for convenience
pub use checker::TypeChecker;
pub use diagnostic::DiagnosticRenderer;
pub use error::{LexError, LexResult};
pub use errors::{Diagnostic, DiagnosticLevel, ErrorCode, IntoDiagnostic};
pub use lexer::{lex, LexMode, Lexer};
pub use span::{FileTable, Span};
pub use token::{Token, TokenWithSpan};
pub use types::ty::{TypeData, TypeId, TypeInterner};
