//! GaleX Language Server Protocol implementation.
//!
//! Provides IDE features via the LSP: diagnostics, autocomplete, hover,
//! go-to-definition, references, rename, code actions, formatting,
//! document symbols, folding, and semantic tokens.

pub mod completions;
pub mod diagnostics;
pub mod document;
pub mod goto;
pub mod hover;
pub mod position;
pub mod quickfix;
pub mod semantic_tokens;
pub mod symbols;
