use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::capabilities::{
    code_actions, completions, definition, diagnostics, folding, hover, inlay_hints, references,
    rename, selection_range, semantic_tokens, signature_help, symbols,
};
use crate::document::DocumentManager;
use crate::parser::SourceParser;

/// The LSP backend that handles all language server requests
pub struct Backend {
    client: Client,
    document_manager: DocumentManager,
    parser: Mutex<SourceParser>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_manager: DocumentManager::new(),
            parser: Mutex::new(SourceParser::default()),
        }
    }

    /// Publish diagnostics for a document
    async fn publish_diagnostics(&self, uri: Url) {
        let diags = if let Some(doc) = self.document_manager.get(&uri) {
            if let Some(ref tree) = doc.tree {
                diagnostics::get_syntax_diagnostics(tree, &doc.content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::get_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "<".to_string(),
                        "/".to_string(),
                    ]),
                    resolve_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::REFACTOR,
                            CodeActionKind::REFACTOR_EXTRACT,
                            CodeActionKind::REFACTOR_REWRITE,
                            CodeActionKind::SOURCE,
                            CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                        ]),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "typescript-language-server-rust".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "TypeScript/JavaScript Language Server initialized!",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        {
            let mut parser = self.parser.lock().unwrap();
            self.document_manager
                .open(uri.clone(), content, version, &mut parser);
        }

        // Publish diagnostics for the newly opened document
        self.publish_diagnostics(uri.clone()).await;

        self.client
            .log_message(MessageType::INFO, format!("Opened document: {}", uri))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        {
            let mut parser = self.parser.lock().unwrap();
            self.document_manager
                .change(&uri, changes, version, &mut parser);
        }

        // Update diagnostics after changes
        self.publish_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_manager.close(&uri);

        // Clear diagnostics for closed document
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;

        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                hover::get_hover(tree, &doc.content, position)
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = &params.text_document.uri;

        let mut ranges = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                folding::get_folding_ranges(tree, &doc.content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Merge consecutive imports
        folding::merge_import_ranges(&mut ranges);

        Ok(Some(ranges))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let uri = &params.text_document.uri;
        let positions = &params.positions;

        let ranges = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                selection_range::get_selection_ranges(tree, positions)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(ranges))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref symbol_table) = doc.symbol_table {
                definition::get_definition(symbol_table, &doc.content, position, uri)
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref symbol_table) = doc.symbol_table {
                let refs = references::get_references(
                    symbol_table,
                    &doc.content,
                    position,
                    uri,
                    include_declaration,
                );
                if refs.is_empty() { None } else { Some(refs) }
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref symbol_table) = doc.symbol_table {
                rename::prepare_rename(symbol_table, &doc.content, position)
                    .map(PrepareRenameResponse::Range)
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref symbol_table) = doc.symbol_table {
                rename::rename_symbol(symbol_table, &doc.content, position, new_name, uri)
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        let symbols = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                symbols::get_document_symbols(tree, &doc.content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let tokens = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                semantic_tokens::get_semantic_tokens(tree, &doc.content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;

        let items = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                if let Some(ref symbol_table) = doc.symbol_table {
                    completions::get_completions(tree, &doc.content, symbol_table, &params)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn completion_resolve(&self, item: CompletionItem) -> Result<CompletionItem> {
        // For now, just return the item as-is
        // In a full implementation, we would fetch additional documentation here
        Ok(item)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let result = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                if let Some(ref symbol_table) = doc.symbol_table {
                    signature_help::get_signature_help(tree, &doc.content, symbol_table, position)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(result)
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        let hints = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref tree) = doc.tree {
                if let Some(ref symbol_table) = doc.symbol_table {
                    inlay_hints::get_inlay_hints(tree, &doc.content, symbol_table, range)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(hints))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let range = params.range;
        let diagnostics = &params.context.diagnostics;

        let actions = if let Some(doc) = self.document_manager.get(uri) {
            if let Some(ref symbol_table) = doc.symbol_table {
                code_actions::get_code_actions(uri, range, diagnostics, symbol_table, &doc.content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Some(actions))
    }
}
