#![doc = include_str!("../README.md")]
use lsp_server::{Connection, IoThreads, Message, Response, ResponseError};
use lsp_textdocument::{FullTextDocument, TextDocuments};
use lsp_types::{
    request::{self, Request},
    Hover, HoverContents, HoverParams, InlayHint, MarkupContent, MarkupKind, OneOf, Position,
    Range, ReferencesOptions, RenameOptions, SemanticToken, SemanticTokenType,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensServerCapabilities,
    ServerCapabilities, SignatureHelpOptions, TextDocumentSyncKind, Uri, WorkDoneProgressOptions,
    WorkspaceFolder, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use miette::{miette, IntoDiagnostic, Result};
use nu_protocol::{
    ast::{Block, PathMember},
    engine::{Command, EngineState, StateDelta, StateWorkingSet},
    DeclId, ModuleId, Span, Type, VarId,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    sync::Mutex,
    time::Duration,
};
use symbols::SymbolCache;
use workspace::{InternalMessage, RangePerDoc};

mod ast;
mod completion;
mod diagnostics;
mod goto;
mod hints;
mod notification;
mod semantic_tokens;
mod signature;
mod symbols;
mod workspace;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Id {
    Variable(VarId),
    Declaration(DeclId),
    Value(Type),
    Module(ModuleId),
    CellPath(VarId, Vec<PathMember>),
    External(String),
}

pub struct LanguageServer {
    connection: Connection,
    io_threads: Option<IoThreads>,
    docs: Arc<Mutex<TextDocuments>>,
    initial_engine_state: EngineState,
    symbol_cache: SymbolCache,
    inlay_hints: BTreeMap<Uri, Vec<InlayHint>>,
    semantic_tokens: BTreeMap<Uri, Vec<SemanticToken>>,
    workspace_folders: BTreeMap<String, WorkspaceFolder>,
    /// for workspace wide requests
    occurrences: BTreeMap<Uri, Vec<Range>>,
    channels: Option<(
        crossbeam_channel::Sender<bool>,
        Arc<crossbeam_channel::Receiver<InternalMessage>>,
    )>,
    /// set to true when text changes
    need_parse: bool,
    /// cache `StateDelta` to avoid repeated parsing
    cached_state_delta: Option<StateDelta>,
}

pub(crate) fn path_to_uri(path: impl AsRef<Path>) -> Uri {
    Uri::from_str(
        url::Url::from_file_path(path)
            .expect("Failed to convert path to Url")
            .as_str(),
    )
    .expect("Failed to convert Url to lsp_types::Uri.")
}

pub(crate) fn uri_to_path(uri: &Uri) -> PathBuf {
    url::Url::from_str(uri.as_str())
        .expect("Failed to convert Uri to Url")
        .to_file_path()
        .expect("Failed to convert Url to path")
}

pub(crate) fn span_to_range(span: &Span, file: &FullTextDocument, offset: usize) -> Range {
    let start = file.position_at(span.start.saturating_sub(offset) as u32);
    let end = file.position_at(span.end.saturating_sub(offset) as u32);
    Range { start, end }
}

impl LanguageServer {
    pub fn initialize_stdio_connection(engine_state: EngineState) -> Result<Self> {
        let (connection, io_threads) = Connection::stdio();
        Self::initialize_connection(connection, Some(io_threads), engine_state)
    }

    fn initialize_connection(
        connection: Connection,
        io_threads: Option<IoThreads>,
        engine_state: EngineState,
    ) -> Result<Self> {
        Ok(Self {
            connection,
            io_threads,
            docs: Arc::new(Mutex::new(TextDocuments::new())),
            initial_engine_state: engine_state,
            symbol_cache: SymbolCache::new(),
            inlay_hints: BTreeMap::new(),
            semantic_tokens: BTreeMap::new(),
            workspace_folders: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            channels: None,
            need_parse: true,
            cached_state_delta: None,
        })
    }

    pub fn serve_requests(mut self) -> Result<()> {
        let work_done_progress_options = WorkDoneProgressOptions {
            work_done_progress: Some(true),
        };
        let server_capabilities = serde_json::to_value(ServerCapabilities {
            completion_provider: Some(lsp_types::CompletionOptions::default()),
            definition_provider: Some(OneOf::Left(true)),
            document_highlight_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Right(ReferencesOptions {
                work_done_progress_options,
            })),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options,
            })),
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                ..Default::default()
            }),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    // NOTE: only internal command names with space supported for now
                    legend: SemanticTokensLegend {
                        token_types: vec![SemanticTokenType::FUNCTION],
                        token_modifiers: vec![],
                    },
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                }),
            ),
            signature_help_provider: Some(SignatureHelpOptions::default()),
            ..Default::default()
        })
        .expect("Must be serializable");
        let init_params = self
            .connection
            .initialize_while(server_capabilities, || {
                !self.initial_engine_state.signals().interrupted()
            })
            .into_diagnostic()?;
        self.initialize_workspace_folders(init_params);

        while !self.initial_engine_state.signals().interrupted() {
            // first check new messages from child thread
            self.handle_internal_messages()?;

            let msg = match self
                .connection
                .receiver
                .recv_timeout(Duration::from_secs(1))
            {
                Ok(msg) => {
                    // cancel execution if other messages received before job done
                    self.cancel_background_thread();
                    msg
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(_) => break,
            };

            match msg {
                Message::Request(request) => {
                    if self
                        .connection
                        .handle_shutdown(&request)
                        .into_diagnostic()?
                    {
                        return Ok(());
                    }

                    let resp = match request.method.as_str() {
                        request::Completion::METHOD => {
                            Self::handle_lsp_request(request, |params| self.complete(params))
                        }
                        request::DocumentHighlightRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.document_highlight(params)
                            })
                        }
                        request::DocumentSymbolRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.document_symbol(params))
                        }
                        request::GotoDefinition::METHOD => {
                            Self::handle_lsp_request(request, |params| self.goto_definition(params))
                        }
                        request::HoverRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.hover(params))
                        }
                        request::InlayHintRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.get_inlay_hints(params))
                        }
                        request::PrepareRenameRequest::METHOD => {
                            let id = request.id.clone();
                            if let Err(e) = self.prepare_rename(request) {
                                self.send_error_message(id, 2, e.to_string())?
                            }
                            continue;
                        }
                        request::References::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.references(params, 5000)
                            })
                        }
                        request::Rename::METHOD => {
                            Self::handle_lsp_request(request, |params| self.rename(params))
                        }
                        request::SemanticTokensFullRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.get_semantic_tokens(params)
                            })
                        }
                        request::SignatureHelpRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.get_signature_help(params)
                            })
                        }
                        request::WorkspaceSymbolRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.workspace_symbol(params)
                            })
                        }
                        _ => {
                            continue;
                        }
                    };

                    self.connection
                        .sender
                        .send(Message::Response(resp))
                        .into_diagnostic()?;
                }
                Message::Response(_) => {}
                Message::Notification(notification) => {
                    if let Some(updated_file) = self.handle_lsp_notification(notification) {
                        self.need_parse = true;
                        self.symbol_cache.mark_dirty(updated_file.clone(), true);
                        self.publish_diagnostics_for_file(updated_file)?;
                    }
                }
            }
        }

        if let Some(io_threads) = self.io_threads {
            io_threads.join().into_diagnostic()?;
        }

        Ok(())
    }

    /// Send a cancel message to a running bg thread
    pub(crate) fn cancel_background_thread(&mut self) {
        if let Some((sender, _)) = &self.channels {
            sender.send(true).ok();
        }
    }

    /// Check results from background thread
    pub(crate) fn handle_internal_messages(&mut self) -> Result<bool> {
        let mut reset = false;
        if let Some((_, receiver)) = &self.channels {
            for im in receiver.try_iter() {
                match im {
                    InternalMessage::RangeMessage(RangePerDoc { uri, ranges }) => {
                        self.occurrences.insert(uri, ranges);
                    }
                    InternalMessage::OnGoing(token, progress) => {
                        self.send_progress_report(token, progress, None)?;
                    }
                    InternalMessage::Finished(token) => {
                        reset = true;
                        self.send_progress_end(token, Some("Finished.".to_string()))?;
                    }
                    InternalMessage::Cancelled(token) => {
                        reset = true;
                        self.send_progress_end(token, Some("interrupted.".to_string()))?;
                    }
                }
            }
        }
        if reset {
            self.channels = None;
        }
        Ok(reset)
    }

    pub(crate) fn new_engine_state(&self) -> EngineState {
        let mut engine_state = self.initial_engine_state.clone();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var(
            "PWD".into(),
            nu_protocol::Value::test_string(cwd.to_string_lossy()),
        );
        // merge the cached `StateDelta` if text not changed
        if !self.need_parse {
            engine_state
                .merge_delta(
                    self.cached_state_delta
                        .to_owned()
                        .expect("Tried to merge a non-existing state delta"),
                )
                .expect("Failed to merge state delta");
        }
        engine_state
    }

    fn cache_parsed_block(&mut self, working_set: &mut StateWorkingSet, block: Arc<Block>) {
        if self.need_parse {
            // TODO: incremental parsing
            // add block to working_set for later references
            working_set.add_block(block.clone());
            self.cached_state_delta = Some(working_set.delta.clone());
            self.need_parse = false;
        }
    }

    pub(crate) fn parse_and_find<'a>(
        &mut self,
        engine_state: &'a mut EngineState,
        uri: &Uri,
        pos: Position,
    ) -> Result<(StateWorkingSet<'a>, Id, Span, usize)> {
        let (block, file_span, working_set) = self
            .parse_file(engine_state, uri, false)
            .ok_or_else(|| miette!("\nFailed to parse current file"))?;

        let docs = match self.docs.lock() {
            Ok(it) => it,
            Err(err) => return Err(miette!(err.to_string())),
        };
        let file = docs
            .get_document(uri)
            .ok_or_else(|| miette!("\nFailed to get document"))?;
        let location = file.offset_at(pos) as usize + file_span.start;
        let (id, span) = ast::find_id(&block, &working_set, &location)
            .ok_or_else(|| miette!("\nFailed to find current name"))?;
        Ok((working_set, id, span, file_span.start))
    }

    pub(crate) fn parse_file<'a>(
        &mut self,
        engine_state: &'a mut EngineState,
        uri: &Uri,
        need_extra_info: bool,
    ) -> Option<(Arc<Block>, Span, StateWorkingSet<'a>)> {
        let mut working_set = StateWorkingSet::new(engine_state);
        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(uri)?;
        let file_path = uri_to_path(uri);
        let file_path_str = file_path.to_str()?;
        let contents = file.get_content(None).as_bytes();
        let _ = working_set.files.push(file_path.clone(), Span::unknown());
        let block = nu_parser::parse(&mut working_set, Some(file_path_str), contents, false);
        let span = working_set.get_span_for_filename(file_path_str)?;
        if need_extra_info {
            let file_inlay_hints =
                Self::extract_inlay_hints(&working_set, &block, span.start, file);
            self.inlay_hints.insert(uri.clone(), file_inlay_hints);
            let file_semantic_tokens =
                Self::extract_semantic_tokens(&working_set, &block, span.start, file);
            self.semantic_tokens
                .insert(uri.clone(), file_semantic_tokens);
        }
        drop(docs);
        self.cache_parsed_block(&mut working_set, block.clone());
        Some((block, span, working_set))
    }

    fn handle_lsp_request<P, H, R>(req: lsp_server::Request, mut param_handler: H) -> Response
    where
        P: serde::de::DeserializeOwned,
        H: FnMut(&P) -> Option<R>,
        R: serde::ser::Serialize,
    {
        match serde_json::from_value::<P>(req.params) {
            Ok(params) => Response {
                id: req.id,
                result: Some(
                    param_handler(&params)
                        .and_then(|response| serde_json::to_value(response).ok())
                        .unwrap_or(serde_json::Value::Null),
                ),
                error: None,
            },

            Err(err) => Response {
                id: req.id,
                result: None,
                error: Some(ResponseError {
                    code: 1,
                    message: err.to_string(),
                    data: None,
                }),
            },
        }
    }

    fn get_decl_description(decl: &dyn Command, skip_description: bool) -> String {
        let mut description = String::new();

        if !skip_description {
            // First description
            description.push_str(&format!("{}\n", decl.description().replace('\r', "")));

            // Additional description
            if !decl.extra_description().is_empty() {
                description.push_str(&format!("\n{}\n", decl.extra_description()));
            }
        }
        // Usage
        description.push_str("---\n### Usage \n```nu\n");
        let signature = decl.signature();
        description.push_str(&Self::get_signature_label(&signature));
        description.push_str("\n```\n");

        // Flags
        if !signature.named.is_empty() {
            description.push_str("\n### Flags\n\n");
            let mut first = true;
            for named in &signature.named {
                if first {
                    first = false;
                } else {
                    description.push('\n');
                }
                description.push_str("  ");
                if let Some(short_flag) = &named.short {
                    description.push_str(&format!("`-{short_flag}`"));
                }
                if !named.long.is_empty() {
                    if named.short.is_some() {
                        description.push_str(", ");
                    }
                    description.push_str(&format!("`--{}`", named.long));
                }
                if let Some(arg) = &named.arg {
                    description.push_str(&format!(" `<{}>`", arg.to_type()));
                }
                if !named.desc.is_empty() {
                    description.push_str(&format!(" - {}", named.desc));
                }
                description.push('\n');
            }
            description.push('\n');
        }

        // Parameters
        if !signature.required_positional.is_empty()
            || !signature.optional_positional.is_empty()
            || signature.rest_positional.is_some()
        {
            description.push_str("\n### Parameters\n\n");
            let mut first = true;
            for required_arg in &signature.required_positional {
                if first {
                    first = false;
                } else {
                    description.push('\n');
                }
                description.push_str(&format!(
                    "  `{}: {}`",
                    required_arg.name,
                    required_arg.shape.to_type()
                ));
                if !required_arg.desc.is_empty() {
                    description.push_str(&format!(" - {}", required_arg.desc));
                }
                description.push('\n');
            }
            for optional_arg in &signature.optional_positional {
                if first {
                    first = false;
                } else {
                    description.push('\n');
                }
                description.push_str(&format!(
                    "  `{}: {}`",
                    optional_arg.name,
                    optional_arg.shape.to_type()
                ));
                if !optional_arg.desc.is_empty() {
                    description.push_str(&format!(" - {}", optional_arg.desc));
                }
                description.push('\n');
            }
            if let Some(arg) = &signature.rest_positional {
                if !first {
                    description.push('\n');
                }
                description.push_str(&format!(" `...{}: {}`", arg.name, arg.shape.to_type()));
                if !arg.desc.is_empty() {
                    description.push_str(&format!(" - {}", arg.desc));
                }
                description.push('\n');
            }
            description.push('\n');
        }

        // Input/output types
        if !signature.input_output_types.is_empty() {
            description.push_str("\n### Input/output types\n");
            description.push_str("\n```nu\n");
            for input_output in &signature.input_output_types {
                description.push_str(&format!(" {} | {}\n", input_output.0, input_output.1));
            }
            description.push_str("\n```\n");
        }

        // Examples
        if !decl.examples().is_empty() {
            description.push_str("### Example(s)\n");
            for example in decl.examples() {
                description.push_str(&format!(
                    "  {}\n```nu\n  {}\n```\n",
                    example.description, example.example
                ));
            }
        }
        description
    }

    fn hover(&mut self, params: &HoverParams) -> Option<Hover> {
        let mut engine_state = self.new_engine_state();

        let path_uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_owned();
        let (working_set, id, _, _) = self
            .parse_and_find(
                &mut engine_state,
                &path_uri,
                params.text_document_position_params.position,
            )
            .ok()?;

        let markdown_hover = |content: String| {
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                // TODO
                range: None,
            })
        };

        match id {
            Id::Variable(var_id) => {
                let var = working_set.get_variable(var_id);
                let value = var
                    .const_val
                    .clone()
                    .and_then(|v| v.coerce_into_string().ok())
                    .unwrap_or(String::from(if var.mutable {
                        "mutable"
                    } else {
                        "immutable"
                    }));
                let contents = format!("```\n{}\n``` \n---\n{}", var.ty, value);
                markdown_hover(contents)
            }
            Id::CellPath(var_id, cell_path) => {
                let var = working_set.get_variable(var_id);
                markdown_hover(
                    var.const_val
                        .clone()
                        .and_then(|val| val.follow_cell_path(&cell_path, false).ok())
                        .map(|val| {
                            let ty = val.get_type().clone();
                            let value_string = val
                                .coerce_into_string()
                                .ok()
                                .map(|s| format!("\n---\n{}", s))
                                .unwrap_or_default();
                            format!("```\n{}\n```{}", ty, value_string)
                        })
                        .unwrap_or("`unknown`".into()),
                )
            }
            Id::Declaration(decl_id) => markdown_hover(Self::get_decl_description(
                working_set.get_decl(decl_id),
                false,
            )),
            Id::Module(module_id) => {
                let description = working_set
                    .get_module_comments(module_id)?
                    .iter()
                    .map(|sp| String::from_utf8_lossy(working_set.get_span_contents(*sp)).into())
                    .collect::<Vec<String>>()
                    .join("\n");
                markdown_hover(description)
            }
            Id::Value(t) => markdown_hover(format!("`{}`", t)),
            Id::External(cmd) => {
                let command_output = if cfg!(windows) {
                    std::process::Command::new("powershell.exe")
                        .args(["-NoProfile", "-Command", "help", &cmd])
                        .output()
                } else {
                    std::process::Command::new("man").arg(&cmd).output()
                };
                let manpage_str = match command_output {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(_) => format!("No command help found for {}", &cmd),
                };
                markdown_hover(manpage_str)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json_diff::assert_json_eq;
    use lsp_types::{
        notification::{
            DidChangeTextDocument, DidOpenTextDocument, Exit, Initialized, Notification,
        },
        request::{HoverRequest, Initialize, Request, Shutdown},
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializedParams, Position,
        TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };
    use nu_protocol::{debugger::WithoutDebug, engine::Stack, PipelineData, ShellError, Value};
    use nu_test_support::fs::fixtures;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    /// Initialize the language server for test purposes
    ///
    /// # Arguments
    /// - `nu_config_code`: Optional user defined `config.nu` that is loaded on start
    /// - `params`: Optional client side capability parameters
    pub(crate) fn initialize_language_server(
        nu_config_code: Option<&str>,
        params: Option<serde_json::Value>,
    ) -> (Connection, Receiver<Result<()>>) {
        let engine_state = nu_cmd_lang::create_default_context();
        let mut engine_state = nu_command::add_shell_command_context(engine_state);
        engine_state.generate_nu_constant();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var(
            "PWD".into(),
            nu_protocol::Value::test_string(cwd.to_string_lossy()),
        );
        if let Some(code) = nu_config_code {
            assert!(merge_input(code.as_bytes(), &mut engine_state, &mut Stack::new()).is_ok());
        }

        let (client_connection, server_connection) = Connection::memory();
        let lsp_server =
            LanguageServer::initialize_connection(server_connection, None, engine_state).unwrap();

        let (send, recv) = mpsc::channel();
        std::thread::spawn(move || send.send(lsp_server.serve_requests()));

        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: Initialize::METHOD.to_string(),
                params: params.unwrap_or(serde_json::Value::Null),
            }))
            .unwrap();
        client_connection
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: Initialized::METHOD.to_string(),
                params: serde_json::to_value(InitializedParams {}).unwrap(),
            }))
            .unwrap();

        let _initialize_response = client_connection
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();

        (client_connection, recv)
    }

    /// merge_input executes the given input into the engine
    /// and merges the state
    fn merge_input(
        input: &[u8],
        engine_state: &mut EngineState,
        stack: &mut Stack,
    ) -> Result<(), ShellError> {
        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(engine_state);

            let block = nu_parser::parse(&mut working_set, None, input, false);

            assert!(working_set.parse_errors.is_empty());

            (block, working_set.render())
        };

        engine_state.merge_delta(delta)?;

        assert!(nu_engine::eval_block::<WithoutDebug>(
            engine_state,
            stack,
            &block,
            PipelineData::Value(Value::nothing(Span::unknown()), None),
        )
        .is_ok());

        // Merge environment into the permanent state
        engine_state.merge_env(stack)
    }

    #[test]
    fn shutdown_on_request() {
        let (client_connection, recv) = initialize_language_server(None, None);

        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: Shutdown::METHOD.to_string(),
                params: serde_json::Value::Null,
            }))
            .unwrap();
        client_connection
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: Exit::METHOD.to_string(),
                params: serde_json::Value::Null,
            }))
            .unwrap();

        assert!(recv.recv_timeout(Duration::from_secs(2)).unwrap().is_ok());
    }

    pub(crate) fn open_unchecked(
        client_connection: &Connection,
        uri: Uri,
    ) -> lsp_server::Notification {
        open(client_connection, uri).unwrap()
    }

    pub(crate) fn open(
        client_connection: &Connection,
        uri: Uri,
    ) -> Result<lsp_server::Notification, String> {
        let text = std::fs::read_to_string(uri_to_path(&uri)).map_err(|e| e.to_string())?;

        client_connection
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: DidOpenTextDocument::METHOD.to_string(),
                params: serde_json::to_value(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri,
                        language_id: String::from("nu"),
                        version: 1,
                        text,
                    },
                })
                .unwrap(),
            }))
            .map_err(|e| e.to_string())?;

        let notification = client_connection
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?;

        if let Message::Notification(n) = notification {
            Ok(n)
        } else {
            Err(String::from("Did not receive a notification from server"))
        }
    }

    pub(crate) fn update(
        client_connection: &Connection,
        uri: Uri,
        text: String,
        range: Option<Range>,
    ) -> lsp_server::Notification {
        client_connection
            .sender
            .send(lsp_server::Message::Notification(
                lsp_server::Notification {
                    method: DidChangeTextDocument::METHOD.to_string(),
                    params: serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: lsp_types::VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range,
                            range_length: None,
                            text,
                        }],
                    })
                    .unwrap(),
                },
            ))
            .unwrap();

        let notification = client_connection
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();

        if let Message::Notification(n) = notification {
            n
        } else {
            panic!();
        }
    }

    pub(crate) fn result_from_message(message: lsp_server::Message) -> serde_json::Value {
        match message {
            Message::Response(Response { result, .. }) => result.expect("Empty result!"),
            _ => panic!("Unexpected message type!"),
        }
    }

    pub(crate) fn send_hover_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: HoverRequest::METHOD.to_string(),
                params: serde_json::to_value(HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();

        client_connection
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
    }

    #[test]
    fn hover_on_variable() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 2, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({ "contents": { "kind": "markdown", "value": "```\ntable\n``` \n---\nimmutable" } })
        );
    }

    #[test]
    fn hover_on_cell_path() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("cell_path.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 4, 3);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nlist<any>\n```")
        );

        let resp = send_hover_request(&client_connection, script.clone(), 4, 7);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nrecord<bar: int>\n```")
        );

        let resp = send_hover_request(&client_connection, script.clone(), 4, 11);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nint\n```\n---\n2")
        );
    }

    #[test]
    fn hover_on_custom_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 3, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Renders some greeting message\n---\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_external_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 6, 2);

        let hover_text = result_from_message(resp)
            .pointer("/contents/value")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        #[cfg(not(windows))]
        assert!(hover_text.contains("SLEEP"));
        #[cfg(windows)]
        assert!(hover_text.contains("Start-Sleep"));
    }

    #[test]
    fn hover_on_str_join() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 5, 8);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Concatenate multiple strings into a single string, with an optional separator between each.\n---\n### Usage \n```nu\n  str join {flags} <separator?>\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `separator: string` - Optional separator to use when creating string.\n\n\n### Input/output types\n\n```nu\n list<any> | string\n string | string\n\n```\n### Example(s)\n  Create a string from input\n```nu\n  ['nu', 'shell'] | str join\n```\n  Create a string from input with a separator\n```nu\n  ['nu', 'shell'] | str join '-'\n```\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_module() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 3, 12);
        let result = result_from_message(resp);

        assert_eq!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"# module doc\""
        );
    }
}
