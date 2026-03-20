//! GaleX Language Server — `gale-lsp` binary.
//!
//! Communicates via stdin/stdout JSON-RPC (LSP protocol).
//! Provides diagnostics, autocomplete, hover, go-to-definition,
//! references, rename, code actions, formatting, symbols, folding,
//! and semantic tokens.

use std::sync::Mutex;

use lsp_types::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use galex::lsp::completions;
use galex::lsp::diagnostics;
use galex::lsp::document::DocumentManager;
use galex::lsp::goto;
use galex::lsp::hover;
use galex::lsp::position as pos_util;
use galex::lsp::quickfix;
use galex::lsp::semantic_tokens;
use galex::lsp::symbols;

struct GaleLsp {
    client: Client,
    documents: Mutex<DocumentManager>,
}

#[tower_lsp::async_trait]
impl LanguageServer for GaleLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".into(),
                        ":".into(),
                        "<".into(),
                        "\"".into(),
                        "/".into(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "GaleX language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let diags = {
            let mut docs = self.documents.lock().unwrap();
            docs.open(
                &params.text_document.uri,
                params.text_document.text,
                params.text_document.version,
            );
            docs.recheck();
            self.collect_diags_for(&docs, &uri)
        };
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let diags = {
            let mut docs = self.documents.lock().unwrap();
            if let Some(change) = params.content_changes.into_iter().last() {
                docs.change(
                    &params.text_document.uri,
                    change.text,
                    params.text_document.version,
                );
                docs.recheck();
            }
            self.collect_diags_for(&docs, &uri)
        };
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        {
            let mut docs = self.documents.lock().unwrap();
            docs.close(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let offset = pos_util::position_to_offset(source, pos);
        let trigger = params
            .context
            .as_ref()
            .and_then(|c| c.trigger_character.as_deref());
        let items = completions::provide_completions(&docs, uri, source, offset, trigger);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let file_id = match docs.get_file_id(uri) {
            Some(id) => id,
            None => return Ok(None),
        };
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let offset = pos_util::position_to_offset(source, pos);
        Ok(hover::provide_hover(&docs, file_id, offset, source))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let file_id = match docs.get_file_id(uri) {
            Some(id) => id,
            None => return Ok(None),
        };
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let offset = pos_util::position_to_offset(source, pos);
        Ok(goto::goto_definition(&docs, uri, file_id, offset).map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let file_id = match docs.get_file_id(uri) {
            Some(id) => id,
            None => return Ok(None),
        };
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let offset = pos_util::position_to_offset(source, pos);
        let refs = goto::find_references(&docs, file_id, offset);
        Ok(if refs.is_empty() { None } else { Some(refs) })
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let file_id = match docs.get_file_id(uri) {
            Some(id) => id,
            None => return Ok(None),
        };
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let offset = pos_util::position_to_offset(source, pos);
        Ok(goto::rename_symbol(
            &docs,
            file_id,
            offset,
            &params.new_name,
        ))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let actions =
            quickfix::provide_code_actions(&params.text_document.uri, &params.context.diagnostics);
        Ok(if actions.is_empty() {
            None
        } else {
            Some(
                actions
                    .into_iter()
                    .map(CodeActionOrCommand::CodeAction)
                    .collect(),
            )
        })
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let docs = self.documents.lock().unwrap();
        let source = match docs.get_source(&params.text_document.uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        Ok(symbols::format_document(source))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document.uri;
        let ast = match docs.get_ast(uri) {
            Some(a) => a,
            None => return Ok(None),
        };
        let source = docs.get_source(uri).unwrap_or("");
        let syms = symbols::document_symbols(ast, source);
        Ok(Some(DocumentSymbolResponse::Nested(syms)))
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document.uri;
        let ast = match docs.get_ast(uri) {
            Some(a) => a,
            None => return Ok(None),
        };
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        Ok(Some(symbols::folding_ranges(ast, source)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let docs = self.documents.lock().unwrap();
        let uri = &params.text_document.uri;
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return Ok(None),
        };
        let tokens = semantic_tokens::provide_semantic_tokens(&docs, source);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }
}

impl GaleLsp {
    /// Collect diagnostics for a URI while the lock is held (synchronous).
    fn collect_diags_for(&self, docs: &DocumentManager, uri: &Url) -> Vec<Diagnostic> {
        let source = match docs.get_source(uri) {
            Some(s) => s,
            None => return vec![],
        };
        let (lex_errors, parse_errors) = docs.get_parse_errors(uri);
        diagnostics::collect_diagnostics(
            lex_errors,
            parse_errors,
            &docs.type_errors,
            &docs.lint_warnings,
            source,
        )
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| GaleLsp {
        client,
        documents: Mutex::new(DocumentManager::new()),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
