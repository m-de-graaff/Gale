//! In-memory document manager for the LSP server.
//!
//! Maintains source text, parsed ASTs, and cached analysis results
//! for all open documents.

use std::collections::HashMap;

use lsp_types::Url;

use crate::ast::Program;
use crate::checker::TypeChecker;
use crate::lint;
use crate::parser;
use crate::span::{FileTable, Span};
use crate::types::constraint::TypeError;

/// Manages open documents and their analysis state.
pub struct DocumentManager {
    /// Open documents keyed by URI.
    documents: HashMap<String, DocumentState>,
    /// File table for span → path mapping.
    pub file_table: FileTable,
    /// Cached merged program (invalidated on any document change).
    cached_merged: Option<Program>,
    /// Cached type checker (after last successful check).
    pub cached_checker: Option<TypeChecker>,
    /// Last computed type errors.
    pub type_errors: Vec<TypeError>,
    /// Last computed lint warnings.
    pub lint_warnings: Vec<lint::LintWarning>,
}

/// State of a single open document.
struct DocumentState {
    file_id: u32,
    source: String,
    ast: Option<Program>,
    lex_errors: Vec<crate::error::LexError>,
    parse_errors: Vec<crate::parser::error::ParseError>,
    version: i32,
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            file_table: FileTable::new(),
            cached_merged: None,
            cached_checker: None,
            type_errors: Vec::new(),
            lint_warnings: Vec::new(),
        }
    }

    /// Handle document open event.
    pub fn open(&mut self, uri: &Url, text: String, version: i32) {
        let path = uri.to_file_path().unwrap_or_default();
        let file_id = self.file_table.find_or_add_file(path);
        let result = parser::parse(&text, file_id);
        let key = uri.to_string();
        self.documents.insert(
            key,
            DocumentState {
                file_id,
                source: text,
                ast: Some(result.program),
                lex_errors: result.lex_errors,
                parse_errors: result.parse_errors,
                version,
            },
        );
        self.invalidate_cache();
    }

    /// Handle document change event (full text sync).
    pub fn change(&mut self, uri: &Url, text: String, version: i32) {
        let key = uri.to_string();
        if let Some(doc) = self.documents.get_mut(&key) {
            let result = parser::parse(&text, doc.file_id);
            doc.source = text;
            doc.ast = Some(result.program);
            doc.lex_errors = result.lex_errors;
            doc.parse_errors = result.parse_errors;
            doc.version = version;
            self.invalidate_cache();
        }
    }

    /// Handle document close event.
    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(&uri.to_string());
        self.invalidate_cache();
    }

    /// Get the source text for a document.
    pub fn get_source(&self, uri: &Url) -> Option<&str> {
        self.documents
            .get(&uri.to_string())
            .map(|d| d.source.as_str())
    }

    /// Get the parsed AST for a document.
    pub fn get_ast(&self, uri: &Url) -> Option<&Program> {
        self.documents
            .get(&uri.to_string())
            .and_then(|d| d.ast.as_ref())
    }

    /// Get the file ID for a document.
    pub fn get_file_id(&self, uri: &Url) -> Option<u32> {
        self.documents.get(&uri.to_string()).map(|d| d.file_id)
    }

    /// Get lex+parse errors for a specific document.
    pub fn get_parse_errors(
        &self,
        uri: &Url,
    ) -> (
        &[crate::error::LexError],
        &[crate::parser::error::ParseError],
    ) {
        match self.documents.get(&uri.to_string()) {
            Some(doc) => (&doc.lex_errors, &doc.parse_errors),
            None => (&[], &[]),
        }
    }

    /// Re-check all open documents.
    ///
    /// Merges all parsed ASTs, runs the type checker and linter,
    /// and caches the results.
    pub fn recheck(&mut self) {
        // Merge all ASTs
        let mut all_items = Vec::new();
        for doc in self.documents.values() {
            if let Some(ref ast) = doc.ast {
                all_items.extend(ast.items.clone());
            }
        }
        let merged = Program {
            items: all_items,
            span: Span::dummy(),
        };

        // Type check
        let mut checker = TypeChecker::new();
        self.type_errors = checker.check_program(&merged);

        // Lint
        self.lint_warnings = lint::lint_program(&merged);

        // Cache
        self.cached_merged = Some(merged);
        self.cached_checker = Some(checker);
    }

    /// Get the cached merged program.
    pub fn merged_program(&self) -> Option<&Program> {
        self.cached_merged.as_ref()
    }

    /// Get all open document URIs.
    pub fn open_uris(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    /// Get the source text for a file by its file ID.
    ///
    /// Looks up the path from the file table, converts to URI, and checks
    /// open documents. Returns `None` if the file is not currently open.
    pub fn get_source_by_file_id(&self, file_id: u32) -> Option<&str> {
        let path = self.file_table.get_path(file_id)?;
        let uri = Url::from_file_path(path).ok()?;
        self.get_source(&uri)
    }

    fn invalidate_cache(&mut self) {
        self.cached_merged = None;
        self.cached_checker = None;
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}
