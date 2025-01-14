#![doc = include_str!("../README.md")]
use ast::find_id;
use lsp_server::{Connection, IoThreads, Message, Response, ResponseError};
use lsp_textdocument::{FullTextDocument, TextDocuments};
use lsp_types::{
    request, request::Request, CompletionItem, CompletionItemKind, CompletionParams,
    CompletionResponse, CompletionTextEdit, Hover, HoverContents, HoverParams, InlayHint, Location,
    MarkupContent, MarkupKind, OneOf, Range, RenameOptions, ServerCapabilities,
    TextDocumentSyncKind, TextEdit, Uri, WorkDoneProgressOptions,
};
use miette::{IntoDiagnostic, Result};
use nu_cli::{NuCompleter, SuggestionKind};
use nu_parser::parse;
use nu_protocol::{
    ast::Block,
    engine::{CachedFile, EngineState, Stack, StateWorkingSet},
    DeclId, ModuleId, Span, Type, Value, VarId,
};
use std::collections::BTreeMap;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use symbols::SymbolCache;
use url::Url;

mod ast;
mod diagnostics;
mod goto;
mod hints;
mod notification;
mod symbols;

#[derive(Debug, Clone)]
enum Id {
    Variable(VarId),
    Declaration(DeclId),
    Value(Type),
    Module(ModuleId),
}

pub struct LanguageServer {
    connection: Connection,
    io_threads: Option<IoThreads>,
    docs: TextDocuments,
    engine_state: EngineState,
    symbol_cache: SymbolCache,
    inlay_hints: BTreeMap<Uri, Vec<InlayHint>>,
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
            docs: TextDocuments::new(),
            engine_state,
            symbol_cache: SymbolCache::new(),
            inlay_hints: BTreeMap::new(),
        })
    }

    pub fn serve_requests(mut self) -> Result<()> {
        let server_capabilities = serde_json::to_value(ServerCapabilities {
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            definition_provider: Some(OneOf::Left(true)),
            hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
            completion_provider: Some(lsp_types::CompletionOptions::default()),
            document_symbol_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            references_provider: Some(OneOf::Left(true)),
            ..Default::default()
        })
        .expect("Must be serializable");
        let _ = self
            .connection
            .initialize_while(server_capabilities, || {
                !self.engine_state.signals().interrupted()
            })
            .into_diagnostic()?;

        while !self.engine_state.signals().interrupted() {
            let msg = match self
                .connection
                .receiver
                .recv_timeout(Duration::from_secs(1))
            {
                Ok(msg) => msg,
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
                        request::GotoDefinition::METHOD => {
                            Self::handle_lsp_request(request, |params| self.goto_definition(params))
                        }
                        request::HoverRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.hover(params))
                        }
                        request::Completion::METHOD => {
                            Self::handle_lsp_request(request, |params| self.complete(params))
                        }
                        request::DocumentSymbolRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.document_symbol(params))
                        }
                        request::WorkspaceSymbolRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| {
                                self.workspace_symbol(params)
                            })
                        }
                        request::InlayHintRequest::METHOD => {
                            Self::handle_lsp_request(request, |params| self.get_inlay_hints(params))
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

    pub fn new_engine_state(&self) -> EngineState {
        let mut engine_state = self.engine_state.clone();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
        engine_state
    }

    pub fn parse_file<'a>(
        &mut self,
        engine_state: &'a mut EngineState,
        uri: &Uri,
        need_hints: bool,
    ) -> Option<(Arc<Block>, usize, StateWorkingSet<'a>, &FullTextDocument)> {
        let mut working_set = StateWorkingSet::new(engine_state);
        let file = self.docs.get_document(uri)?;
        let file_path = uri_to_path(uri);
        let file_path_str = file_path.to_str()?;
        let contents = file.get_content(None).as_bytes();
        let _ = working_set.files.push(file_path.clone(), Span::unknown());
        let block = parse(&mut working_set, Some(file_path_str), contents, false);
        let offset = working_set.get_span_for_filename(file_path_str)?.start;
        // TODO: merge delta back to engine_state?
        // self.engine_state.merge_delta(working_set.render());

        if need_hints {
            let file_inlay_hints = self.extract_inlay_hints(&working_set, &block, offset, file);
            self.inlay_hints.insert(uri.clone(), file_inlay_hints);
        }
        Some((block, offset, working_set, file))
    }

    fn get_location_by_span<'a>(
        &self,
        files: impl Iterator<Item = &'a CachedFile>,
        span: &Span,
    ) -> Option<Location> {
        for cached_file in files.into_iter() {
            if cached_file.covered_span.contains(span.start) {
                let path = Path::new(&*cached_file.name);
                if !(path.exists() && path.is_file()) {
                    return None;
                }
                let target_uri = path_to_uri(path);
                if let Some(doc) = self.docs.get_document(&target_uri) {
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, doc, cached_file.covered_span.start),
                    });
                } else {
                    // in case where the document is not opened yet, typically included by `nu -I`
                    let temp_doc = FullTextDocument::new(
                        "nu".to_string(),
                        0,
                        String::from_utf8((*cached_file.content).to_vec()).expect("Invalid UTF-8"),
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
        let (block, file_offset, working_set, file) =
            self.parse_file(&mut engine_state, &path_uri, false)?;
        let location =
            file.offset_at(params.text_document_position_params.position) as usize + file_offset;
        let id = find_id(&block, &working_set, &location)?;

        match id {
            Id::Variable(var_id) => {
                let var = working_set.get_variable(var_id);
                let contents = format!("{}{}", if var.mutable { "mutable " } else { "" }, var.ty);

                Some(Hover {
                    contents: HoverContents::Scalar(lsp_types::MarkedString::String(contents)),
                    // TODO
                    range: None,
                })
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

                Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: description,
                    }),
                    // TODO
                    range: None,
                })
            }
            Id::Value(t) => {
                Some(Hover {
                    contents: HoverContents::Scalar(lsp_types::MarkedString::String(t.to_string())),
                    // TODO
                    range: None,
                })
            }
            _ => None,
        }
    }

    fn complete(&mut self, params: &CompletionParams) -> Option<CompletionResponse> {
        let path_uri = params.text_document_position.text_document.uri.to_owned();
        let file = self.docs.get_document(&path_uri)?;

        let mut completer =
            NuCompleter::new(Arc::new(self.engine_state.clone()), Arc::new(Stack::new()));

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

    pub fn initialize_language_server() -> (Connection, Receiver<Result<()>>) {
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
                params: serde_json::to_value(InitializeParams {
                    capabilities: lsp_types::ClientCapabilities {
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .unwrap(),
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
        let (client_connection, recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
            serde_json::json!({
                "contents": "table"
            })
        );
    }

    #[test]
    fn hover_on_custom_command() {
        let (client_connection, _recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
        let (client_connection, _recv) = initialize_language_server();

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
