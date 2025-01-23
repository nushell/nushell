#![doc = include_str!("../README.md")]
use ast::find_id;
use crossbeam_channel::{Receiver, Sender};
use lsp_server::{Connection, IoThreads, Message, Response, ResponseError};
use lsp_textdocument::{FullTextDocument, TextDocuments};
use lsp_types::{
    request::{self, Request},
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
    Hover, HoverContents, HoverParams, InlayHint, Location, MarkupContent, MarkupKind, OneOf,
    Position, Range, ReferencesOptions, RenameOptions, ServerCapabilities, TextDocumentSyncKind,
    TextEdit, Uri, WorkDoneProgressOptions, WorkspaceFolder, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use miette::{miette, IntoDiagnostic, Result};
use nu_cli::{NuCompleter, SuggestionKind};
use nu_parser::parse;
use nu_protocol::{
    ast::Block,
    engine::{CachedFile, EngineState, Stack, StateDelta, StateWorkingSet},
    DeclId, ModuleId, Span, Type, Value, VarId,
};
use std::{collections::BTreeMap, sync::Mutex};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use symbols::SymbolCache;
use url::Url;
use workspace::{InternalMessage, RangePerDoc};

mod ast;
mod diagnostics;
mod goto;
mod hints;
mod notification;
mod symbols;
mod workspace;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Id {
    Variable(VarId),
    Declaration(DeclId),
    Value(Type),
    Module(ModuleId),
}

pub struct LanguageServer {
    connection: Connection,
    io_threads: Option<IoThreads>,
    docs: Arc<Mutex<TextDocuments>>,
    initial_engine_state: EngineState,
    symbol_cache: SymbolCache,
    inlay_hints: BTreeMap<Uri, Vec<InlayHint>>,
    workspace_folders: BTreeMap<String, WorkspaceFolder>,
    /// for workspace wide requests
    occurrences: BTreeMap<Uri, Vec<Range>>,
    channels: Option<(Sender<bool>, Arc<Receiver<InternalMessage>>)>,
    /// set to true when text changes
    need_parse: bool,
    /// cache `StateDelta` to avoid repeated parsing
    cached_state_delta: Option<StateDelta>,
}

pub fn path_to_uri(path: impl AsRef<Path>) -> Uri {
    Uri::from_str(
        Url::from_file_path(path)
            .expect("Failed to convert path to Url")
            .as_str(),
    )
    .expect("Failed to convert Url to lsp_types::Uri.")
}

pub fn uri_to_path(uri: &Uri) -> PathBuf {
    Url::from_str(uri.as_str())
        .expect("Failed to convert Uri to Url")
        .to_file_path()
        .expect("Failed to convert Url to path")
}

pub fn span_to_range(span: &Span, file: &FullTextDocument, offset: usize) -> Range {
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
            ..Default::default()
        })
        .expect("Must be serializable");
        let init_params = self
            .connection
            .initialize_while(server_capabilities, || {
                !self.initial_engine_state.signals().interrupted()
            })
            .into_diagnostic()?;
        self.initialize_workspace_folders(init_params)?;

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
                        request::DocumentSymbolRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.document_symbol(params))
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
    pub fn cancel_background_thread(&mut self) {
        if let Some((sender, _)) = &self.channels {
            sender.send(true).ok();
        }
    }

    /// Check results from background thread
    pub fn handle_internal_messages(&mut self) -> Result<bool> {
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

    pub fn new_engine_state(&self) -> EngineState {
        let mut engine_state = self.initial_engine_state.clone();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
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

    pub fn parse_and_find<'a>(
        &mut self,
        engine_state: &'a mut EngineState,
        uri: &Uri,
        pos: Position,
    ) -> Result<(StateWorkingSet<'a>, Id, Span, usize)> {
        let (block, file_span, mut working_set) = self
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
        let (id, span) = find_id(&block, &working_set, &location)
            .ok_or_else(|| miette!("\nFailed to find current name"))?;
        // add block to working_set for later references
        working_set.add_block(block);
        Ok((working_set, id, span, file_span.start))
    }

    pub fn parse_file<'a>(
        &mut self,
        engine_state: &'a mut EngineState,
        uri: &Uri,
        need_hints: bool,
    ) -> Option<(Arc<Block>, Span, StateWorkingSet<'a>)> {
        let mut working_set = StateWorkingSet::new(engine_state);
        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(uri)?;
        let file_path = uri_to_path(uri);
        let file_path_str = file_path.to_str()?;
        let contents = file.get_content(None).as_bytes();
        let _ = working_set.files.push(file_path.clone(), Span::unknown());
        let block = parse(&mut working_set, Some(file_path_str), contents, false);
        let span = working_set.get_span_for_filename(file_path_str)?;
        if need_hints {
            let file_inlay_hints = self.extract_inlay_hints(&working_set, &block, span.start, file);
            self.inlay_hints.insert(uri.clone(), file_inlay_hints);
        }
        if self.need_parse {
            // TODO: incremental parsing
            self.cached_state_delta = Some(working_set.delta.clone());
            self.need_parse = false;
        }
        Some((block, span, working_set))
    }

    fn get_location_by_span<'a>(
        &self,
        files: impl Iterator<Item = &'a CachedFile>,
        span: &Span,
    ) -> Option<Location> {
        for cached_file in files.into_iter() {
            if cached_file.covered_span.contains(span.start) {
                let path = Path::new(&*cached_file.name);
                if !path.is_file() {
                    return None;
                }
                let target_uri = path_to_uri(path);
                if let Some(file) = self.docs.lock().ok()?.get_document(&target_uri) {
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, file, cached_file.covered_span.start),
                    });
                } else {
                    // in case where the document is not opened yet, typically included by `nu -I`
                    let temp_doc = FullTextDocument::new(
                        "nu".to_string(),
                        0,
                        String::from_utf8_lossy(cached_file.content.as_ref()).to_string(),
                    );
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, &temp_doc, cached_file.covered_span.start),
                    });
                }
            }
        }
        None
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
                let contents =
                    format!("{} `{}`", if var.mutable { "mutable " } else { "" }, var.ty);
                markdown_hover(contents)
            }
            Id::Declaration(decl_id) => {
                let decl = working_set.get_decl(decl_id);

                let mut description = String::new();

                // First description
                description.push_str(&format!("{}\n", decl.description().replace('\r', "")));

                // Additional description
                if !decl.extra_description().is_empty() {
                    description.push_str(&format!("\n{}\n", decl.extra_description()));
                }

                // Usage
                description.push_str("-----\n### Usage \n```nu\n");
                let signature = decl.signature();
                description.push_str(&format!("  {}", signature.name));
                if !signature.named.is_empty() {
                    description.push_str(" {flags}");
                }
                for required_arg in &signature.required_positional {
                    description.push_str(&format!(" <{}>", required_arg.name));
                }
                for optional_arg in &signature.optional_positional {
                    description.push_str(&format!(" <{}?>", optional_arg.name));
                }
                if let Some(arg) = &signature.rest_positional {
                    description.push_str(&format!(" <...{}>", arg.name));
                }
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
                        description.push_str(&format!(
                            " `...{}: {}`",
                            arg.name,
                            arg.shape.to_type()
                        ));
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
                        description
                            .push_str(&format!(" {} | {}\n", input_output.0, input_output.1));
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
                markdown_hover(description)
            }
            Id::Module(module_id) => {
                let mut description = String::new();
                for cmt_span in working_set.get_module_comments(module_id)? {
                    description.push_str(
                        String::from_utf8_lossy(working_set.get_span_contents(*cmt_span)).as_ref(),
                    );
                    description.push('\n');
                }
                markdown_hover(description)
            }
            Id::Value(t) => markdown_hover(format!("`{}`", t)),
        }
    }

    fn complete(&mut self, params: &CompletionParams) -> Option<CompletionResponse> {
        let path_uri = params.text_document_position.text_document.uri.to_owned();
        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(&path_uri)?;

        let mut completer = NuCompleter::new(
            Arc::new(self.initial_engine_state.clone()),
            Arc::new(Stack::new()),
        );

        let location = file.offset_at(params.text_document_position.position) as usize;
        let results = completer.fetch_completions_at(&file.get_content(None)[..location], location);
        if results.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(
                results
                    .into_iter()
                    .map(|r| {
                        let mut start = params.text_document_position.position;
                        start.character -= (r.suggestion.span.end - r.suggestion.span.start) as u32;

                        CompletionItem {
                            label: r.suggestion.value.clone(),
                            detail: r.suggestion.description,
                            kind: Self::lsp_completion_item_kind(r.kind),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                range: Range {
                                    start,
                                    end: params.text_document_position.position,
                                },
                                new_text: r.suggestion.value,
                            })),
                            ..Default::default()
                        }
                    })
                    .collect(),
            ))
        }
    }

    fn lsp_completion_item_kind(
        suggestion_kind: Option<SuggestionKind>,
    ) -> Option<CompletionItemKind> {
        suggestion_kind.and_then(|suggestion_kind| match suggestion_kind {
            SuggestionKind::Type(t) => match t {
                nu_protocol::Type::String => Some(CompletionItemKind::VARIABLE),
                _ => None,
            },
            SuggestionKind::Command(c) => match c {
                nu_protocol::engine::CommandType::Keyword => Some(CompletionItemKind::KEYWORD),
                nu_protocol::engine::CommandType::Builtin => Some(CompletionItemKind::FUNCTION),
                _ => None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json_diff::{assert_json_eq, assert_json_include};
    use lsp_types::{
        notification::{
            DidChangeTextDocument, DidOpenTextDocument, Exit, Initialized, Notification,
        },
        request::{Completion, HoverRequest, Initialize, Request, Shutdown},
        CompletionParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams,
        InitializedParams, PartialResultParams, Position, TextDocumentContentChangeEvent,
        TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
        WorkDoneProgressParams,
    };
    use nu_test_support::fs::fixtures;
    use std::sync::mpsc::Receiver;

    pub fn initialize_language_server(
        params: Option<InitializeParams>,
    ) -> (Connection, Receiver<Result<()>>) {
        use std::sync::mpsc;
        let (client_connection, server_connection) = Connection::memory();
        let engine_state = nu_cmd_lang::create_default_context();
        let engine_state = nu_command::add_shell_command_context(engine_state);
        let lsp_server =
            LanguageServer::initialize_connection(server_connection, None, engine_state).unwrap();

        let (send, recv) = mpsc::channel();
        std::thread::spawn(move || send.send(lsp_server.serve_requests()));

        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: Initialize::METHOD.to_string(),
                params: serde_json::to_value(params.unwrap_or_default()).unwrap(),
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
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();

        (client_connection, recv)
    }

    #[test]
    fn shutdown_on_request() {
        let (client_connection, recv) = initialize_language_server(None);

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

        assert!(recv
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
            .is_ok());
    }

    pub fn open_unchecked(client_connection: &Connection, uri: Uri) -> lsp_server::Notification {
        open(client_connection, uri).unwrap()
    }

    pub fn open(
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

    pub fn update(
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

    pub fn send_hover_request(
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
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
    }

    #[test]
    fn hover_on_variable() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 2, 0);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({ "contents": { "kind": "markdown", "value": " `table`" } })
        );
    }

    #[test]
    fn hover_on_custom_command() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 3, 0);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Renders some greeting message\n-----\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_str_join() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 5, 8);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Concatenate multiple strings into a single string, with an optional separator between each.\n-----\n### Usage \n```nu\n  str join {flags} <separator?>\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `separator: string` - Optional separator to use when creating string.\n\n\n### Input/output types\n\n```nu\n list<any> | string\n string | string\n\n```\n### Example(s)\n  Create a string from input\n```nu\n  ['nu', 'shell'] | str join\n```\n  Create a string from input with a separator\n```nu\n  ['nu', 'shell'] | str join '-'\n```\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_module() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 3, 12);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_eq!(
            result
                .unwrap()
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"# module doc\\n\""
        );
    }

    fn send_complete_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: Completion::METHOD.to_string(),
                params: serde_json::to_value(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: None,
                })
                .unwrap(),
            }))
            .unwrap();

        client_connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
    }

    #[test]
    fn complete_on_variable() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_complete_request(&client_connection, script, 2, 9);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!([
                {
                    "label": "$greeting",
                    "textEdit": {
                    "newText": "$greeting",
                    "range": {
                    "start": { "character": 5, "line": 2 },
                "end": { "character": 9, "line": 2 }
            }
            },
                "kind": 6
            }
            ])
        );
    }

    #[test]
    fn complete_command_with_space() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_complete_request(&client_connection, script, 0, 8);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!([
                {
                    "label": "config nu",
                    "detail": "Edit nu configurations.",
                    "textEdit": {
                    "range": {
                    "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 8 },
            },
                "newText": "config nu"
            },
                "kind": 3
            }
            ])
        );
    }

    #[test]
    fn complete_command_with_line() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("utf_pipeline.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_complete_request(&client_connection, script, 0, 13);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!([
                {
                    "label": "str trim",
                    "detail": "Trim whitespace or specific character.",
                    "textEdit": {
                    "range": {
                    "start": { "line": 0, "character": 8 },
                "end": { "line": 0, "character": 13 },
            },
                "newText": "str trim"
            },
                "kind": 3
            }
            ])
        );
    }

    #[test]
    fn complete_keyword() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("keyword.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_complete_request(&client_connection, script, 0, 2);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_include!(
            actual: result,
            expected: serde_json::json!([
                {
                    "label": "overlay",
                    "textEdit": {
                    "newText": "overlay",
                    "range": {
                    "start": { "character": 0, "line": 0 },
                "end": { "character": 2, "line": 0 }
            }
            },
                "kind": 14
            },
            ])
        );
    }
}
