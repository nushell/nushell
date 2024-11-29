#![doc = include_str!("../README.md")]
use lsp_server::{Connection, IoThreads, Message, Response, ResponseError};
use lsp_types::{
    request::{Completion, GotoDefinition, HoverRequest, Request},
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams, Location,
    MarkupContent, MarkupKind, OneOf, Position, PositionEncodingKind, Range, ServerCapabilities,
    TextDocumentSyncKind, TextEdit, Url,
};
use miette::{IntoDiagnostic, Result};
use nu_cli::{NuCompleter, SuggestionKind};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    engine::{CachedFile, EngineState, Stack, StateWorkingSet},
    DeclId, Span, Value, VarId,
};
use ropey::Rope;
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

mod diagnostics;
mod notification;

#[derive(Debug)]
enum Id {
    Variable(VarId),
    Declaration(DeclId),
    Value(FlatShape),
}

pub struct LanguageServer {
    connection: Connection,
    io_threads: Option<IoThreads>,
    ropes: BTreeMap<PathBuf, Rope>,
    position_encoding: PositionEncodingKind,
}

impl LanguageServer {
    pub fn initialize_stdio_connection() -> Result<Self> {
        let (connection, io_threads) = Connection::stdio();
        Self::initialize_connection(connection, Some(io_threads))
    }

    fn initialize_connection(
        connection: Connection,
        io_threads: Option<IoThreads>,
    ) -> Result<Self> {
        Ok(Self {
            connection,
            io_threads,
            ropes: BTreeMap::new(),
            position_encoding: PositionEncodingKind::UTF16,
        })
    }

    fn get_offset_encoding(&self, initialization_params: serde_json::Value) -> String {
        initialization_params
            .pointer("/capabilities/offsetEncoding/0")
            .unwrap_or(
                initialization_params
                    .pointer("/capabilities/offset_encoding/0")
                    .unwrap_or(&json!("utf-16")),
            )
            .to_string()
    }

    pub fn serve_requests(mut self, engine_state: EngineState) -> Result<()> {
        let server_capabilities = serde_json::to_value(ServerCapabilities {
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            definition_provider: Some(OneOf::Left(true)),
            hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
            completion_provider: Some(lsp_types::CompletionOptions::default()),
            ..Default::default()
        })
        .expect("Must be serializable");

        let initialization_params = self
            .connection
            .initialize_while(server_capabilities, || {
                !engine_state.signals().interrupted()
            })
            .into_diagnostic()?;
        self.position_encoding =
            PositionEncodingKind::from(self.get_offset_encoding(initialization_params));

        while !engine_state.signals().interrupted() {
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

                    let mut engine_state = engine_state.clone();
                    let resp = match request.method.as_str() {
                        GotoDefinition::METHOD => Self::handle_lsp_request(
                            &mut engine_state,
                            request,
                            |engine_state, params| self.goto_definition(engine_state, params),
                        ),
                        HoverRequest::METHOD => Self::handle_lsp_request(
                            &mut engine_state,
                            request,
                            |engine_state, params| self.hover(engine_state, params),
                        ),
                        Completion::METHOD => Self::handle_lsp_request(
                            &mut engine_state,
                            request,
                            |engine_state, params| self.complete(engine_state, params),
                        ),
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
                        let mut engine_state = engine_state.clone();
                        self.publish_diagnostics_for_file(updated_file, &mut engine_state)?;
                    }
                }
            }
        }

        if let Some(io_threads) = self.io_threads {
            io_threads.join().into_diagnostic()?;
        }

        Ok(())
    }

    fn handle_lsp_request<P, H, R>(
        engine_state: &mut EngineState,
        req: lsp_server::Request,
        mut param_handler: H,
    ) -> Response
    where
        P: serde::de::DeserializeOwned,
        H: FnMut(&mut EngineState, &P) -> Option<R>,
        R: serde::ser::Serialize,
    {
        match serde_json::from_value::<P>(req.params) {
            Ok(params) => Response {
                id: req.id,
                result: Some(
                    param_handler(engine_state, &params)
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

    fn span_to_range(
        span: &Span,
        rope_of_file: &Rope,
        offset: usize,
        position_encoding: &PositionEncodingKind,
    ) -> Range {
        let start = Self::lsp_byte_offset_to_utf_cu_position(
            span.start.saturating_sub(offset),
            rope_of_file,
            position_encoding,
        );
        let end = Self::lsp_byte_offset_to_utf_cu_position(
            span.end.saturating_sub(offset),
            rope_of_file,
            position_encoding,
        );
        Range { start, end }
    }

    fn lsp_byte_offset_to_utf_cu_position(
        offset: usize,
        rope_of_file: &Rope,
        position_encoding: &PositionEncodingKind,
    ) -> Position {
        let line = rope_of_file.try_byte_to_line(offset).unwrap_or(0);
        match position_encoding.as_str() {
            "\"utf-8\"" => {
                let character = offset - rope_of_file.line_to_byte(line);
                Position {
                    line: line as u32,
                    character: character as u32,
                }
            }
            _ => {
                let character = rope_of_file.char_to_utf16_cu(rope_of_file.byte_to_char(offset))
                    - rope_of_file.char_to_utf16_cu(rope_of_file.line_to_char(line));
                Position {
                    line: line as u32,
                    character: character as u32,
                }
            }
        }
    }

    fn utf16_cu_position_to_char(rope_of_file: &Rope, position: &Position) -> usize {
        let line_utf_idx =
            rope_of_file.char_to_utf16_cu(rope_of_file.line_to_char(position.line as usize));
        rope_of_file.utf16_cu_to_char(line_utf_idx + position.character as usize)
    }

    pub fn lsp_position_to_location(
        position: &Position,
        rope_of_file: &Rope,
        position_encoding: &PositionEncodingKind,
    ) -> usize {
        match position_encoding.as_str() {
            "\"utf-8\"" => rope_of_file.byte_to_char(
                rope_of_file.line_to_byte(position.line as usize) + position.character as usize,
            ),
            _ => Self::utf16_cu_position_to_char(rope_of_file, position),
        }
    }

    fn lsp_position_to_byte_offset(&self, position: &Position, rope_of_file: &Rope) -> usize {
        match self.position_encoding.as_str() {
            "\"utf-8\"" => {
                rope_of_file.line_to_byte(position.line as usize) + position.character as usize
            }
            _ => rope_of_file
                .try_char_to_byte(Self::utf16_cu_position_to_char(rope_of_file, position))
                .expect("Character index out of range!"),
        }
    }

    fn find_id(
        working_set: &mut StateWorkingSet,
        path: &Path,
        file: &Rope,
        location: usize,
    ) -> Option<(Id, usize, Span)> {
        let file_path = path.to_string_lossy();

        // TODO: think about passing down the rope into the working_set
        let contents = file.bytes().collect::<Vec<u8>>();
        let block = parse(working_set, Some(&file_path), &contents, false);
        let flattened = flatten_block(working_set, &block);

        let offset = working_set.get_span_for_filename(&file_path)?.start;
        let location = location + offset;

        for (span, shape) in flattened {
            if location >= span.start && location < span.end {
                match &shape {
                    FlatShape::Variable(var_id) | FlatShape::VarDecl(var_id) => {
                        return Some((Id::Variable(*var_id), offset, span));
                    }
                    FlatShape::InternalCall(decl_id) => {
                        return Some((Id::Declaration(*decl_id), offset, span));
                    }
                    _ => return Some((Id::Value(shape), offset, span)),
                }
            }
        }
        None
    }

    fn rope<'a, 'b: 'a>(&'b self, file_url: &Url) -> Option<(&'a Rope, &'a PathBuf)> {
        let file_path = file_url.to_file_path().ok()?;

        self.ropes
            .get_key_value(&file_path)
            .map(|(path, rope)| (rope, path))
    }

    fn read_in_file<'a>(
        &self,
        engine_state: &'a mut EngineState,
        file_url: &Url,
    ) -> Option<(&Rope, &PathBuf, StateWorkingSet<'a>)> {
        let (file, path) = self.rope(file_url)?;

        engine_state.file = Some(path.to_owned());

        let working_set = StateWorkingSet::new(engine_state);

        Some((file, path, working_set))
    }

    fn rope_file_from_cached_file(&mut self, cached_file: &CachedFile) -> Result<(Url, &Rope), ()> {
        let uri = Url::from_file_path(&*cached_file.name)?;
        let rope_of_file = self.ropes.entry(uri.to_file_path()?).or_insert_with(|| {
            let raw_string = String::from_utf8_lossy(&cached_file.content);
            Rope::from_str(&raw_string)
        });
        Ok((uri, rope_of_file))
    }

    fn goto_definition(
        &mut self,
        engine_state: &mut EngineState,
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        let (file, path, mut working_set) = self.read_in_file(
            engine_state,
            &params.text_document_position_params.text_document.uri,
        )?;

        let (id, _, _) = Self::find_id(
            &mut working_set,
            path,
            file,
            self.lsp_position_to_byte_offset(&params.text_document_position_params.position, file),
        )?;

        match id {
            Id::Declaration(decl_id) => {
                if let Some(block_id) = working_set.get_decl(decl_id).block_id() {
                    let block = working_set.get_block(block_id);
                    if let Some(span) = &block.span {
                        for cached_file in working_set.files() {
                            if cached_file.covered_span.contains(span.start) {
                                let position_encoding = self.position_encoding.clone();
                                let (uri, rope_of_file) =
                                    self.rope_file_from_cached_file(cached_file).ok()?;
                                return Some(GotoDefinitionResponse::Scalar(Location {
                                    uri,
                                    range: Self::span_to_range(
                                        span,
                                        rope_of_file,
                                        cached_file.covered_span.start,
                                        &position_encoding,
                                    ),
                                }));
                            }
                        }
                    }
                }
            }
            Id::Variable(var_id) => {
                let var = working_set.get_variable(var_id);
                for cached_file in working_set.files() {
                    if cached_file
                        .covered_span
                        .contains(var.declaration_span.start)
                    {
                        let position_encoding = self.position_encoding.clone();
                        let (uri, rope_of_file) =
                            self.rope_file_from_cached_file(cached_file).ok()?;
                        return Some(GotoDefinitionResponse::Scalar(Location {
                            uri,
                            range: Self::span_to_range(
                                &var.declaration_span,
                                rope_of_file,
                                cached_file.covered_span.start,
                                &position_encoding,
                            ),
                        }));
                    }
                }
            }
            Id::Value(_) => {}
        }
        None
    }

    fn hover(&mut self, engine_state: &mut EngineState, params: &HoverParams) -> Option<Hover> {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        let (file, path, mut working_set) = self.read_in_file(
            engine_state,
            &params.text_document_position_params.text_document.uri,
        )?;

        let (id, _, _) = Self::find_id(
            &mut working_set,
            path,
            file,
            self.lsp_position_to_byte_offset(&params.text_document_position_params.position, file),
        )?;

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
                description.push_str("### Usage \n```nu\n");
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
            Id::Value(shape) => {
                let hover = String::from(match shape {
                    FlatShape::Binary => "binary",
                    FlatShape::Block => "block",
                    FlatShape::Bool => "bool",
                    FlatShape::Closure => "closure",
                    FlatShape::DateTime => "datetime",
                    FlatShape::Directory => "directory",
                    FlatShape::External => "external",
                    FlatShape::ExternalArg => "external arg",
                    FlatShape::Filepath => "file path",
                    FlatShape::Flag => "flag",
                    FlatShape::Float => "float",
                    FlatShape::GlobPattern => "glob pattern",
                    FlatShape::Int => "int",
                    FlatShape::Keyword => "keyword",
                    FlatShape::List => "list",
                    FlatShape::MatchPattern => "match-pattern",
                    FlatShape::Nothing => "nothing",
                    FlatShape::Range => "range",
                    FlatShape::Record => "record",
                    FlatShape::String => "string",
                    FlatShape::StringInterpolation => "string interpolation",
                    FlatShape::Table => "table",
                    _ => {
                        return None;
                    }
                });

                Some(Hover {
                    contents: HoverContents::Scalar(lsp_types::MarkedString::String(hover)),
                    // TODO
                    range: None,
                })
            }
        }
    }

    fn complete(
        &mut self,
        engine_state: &mut EngineState,
        params: &CompletionParams,
    ) -> Option<CompletionResponse> {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        let (rope_of_file, _, _) = self.read_in_file(
            engine_state,
            &params.text_document_position.text_document.uri,
        )?;

        let mut completer =
            NuCompleter::new(Arc::new(engine_state.clone()), Arc::new(Stack::new()));

        let location =
            self.lsp_position_to_byte_offset(&params.text_document_position.position, rope_of_file);
        let results =
            completer.fetch_completions_at(&rope_of_file.to_string()[..location], location);
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
        request::{Completion, GotoDefinition, HoverRequest, Initialize, Request, Shutdown},
        CompletionParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
        GotoDefinitionParams, InitializeParams, InitializedParams, PartialResultParams,
        TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, Url, WorkDoneProgressParams,
    };
    use nu_test_support::fs::{fixtures, root};
    use std::sync::mpsc::Receiver;

    pub fn initialize_language_server(
        client_offset_encoding: Option<Vec<String>>,
    ) -> (Connection, Receiver<Result<()>>) {
        use std::sync::mpsc;
        let (client_connection, server_connection) = Connection::memory();
        let lsp_server = LanguageServer::initialize_connection(server_connection, None).unwrap();

        let (send, recv) = mpsc::channel();
        std::thread::spawn(move || {
            let engine_state = nu_cmd_lang::create_default_context();
            let engine_state = nu_command::add_shell_command_context(engine_state);
            send.send(lsp_server.serve_requests(engine_state))
        });

        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: Initialize::METHOD.to_string(),
                params: serde_json::to_value(InitializeParams {
                    capabilities: lsp_types::ClientCapabilities {
                        offset_encoding: client_offset_encoding,
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

    #[test]
    fn goto_definition_for_none_existing_file() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut none_existent_path = root();
        none_existent_path.push("none-existent.nu");

        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: GotoDefinition::METHOD.to_string(),
                params: serde_json::to_value(GotoDefinitionParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier {
                            uri: Url::from_file_path(none_existent_path).unwrap(),
                        },
                        position: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();

        let resp = client_connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(result, serde_json::json!(null));
    }

    pub fn open_unchecked(client_connection: &Connection, uri: Url) -> lsp_server::Notification {
        open(client_connection, uri).unwrap()
    }

    pub fn open(
        client_connection: &Connection,
        uri: Url,
    ) -> Result<lsp_server::Notification, String> {
        let text =
            std::fs::read_to_string(uri.to_file_path().unwrap()).map_err(|e| e.to_string())?;

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
        uri: Url,
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
                            uri,
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

    fn goto_definition(
        client_connection: &Connection,
        uri: Url,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: GotoDefinition::METHOD.to_string(),
                params: serde_json::to_value(GotoDefinitionParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
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
    fn goto_definition_of_variable() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("var.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = goto_definition(&client_connection, script.clone(), 2, 12);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
               "uri": script,
               "range": {
                  "start": { "line": 0, "character": 4 },
                  "end": { "line": 0, "character": 12 }
               }
            })
        );
    }

    #[test]
    fn goto_definition_of_command() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = goto_definition(&client_connection, script.clone(), 4, 1);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
               "uri": script,
               "range": {
                  "start": { "line": 0, "character": 17 },
                  "end": { "line": 2, "character": 1 }
               }
            })
        );
    }

    #[test]
    fn goto_definition_of_command_utf8() {
        let (client_connection, _recv) =
            initialize_language_server(Some(vec!["utf-8".to_string(), "utf-16".to_string()]));

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command_unicode.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = goto_definition(&client_connection, script.clone(), 4, 1);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
               "uri": script,
               "range": {
                  "start": { "line": 0, "character": 28 },
                  "end": { "line": 2, "character": 1 }
               }
            })
        );
    }

    #[test]
    fn goto_definition_of_command_utf16() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command_unicode.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = goto_definition(&client_connection, script.clone(), 4, 1);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
               "uri": script,
               "range": {
                  "start": { "line": 0, "character": 19 },
                  "end": { "line": 2, "character": 1 }
               }
            })
        );
    }

    #[test]
    fn goto_definition_of_command_parameter() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = goto_definition(&client_connection, script.clone(), 1, 14);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
               "uri": script,
               "range": {
                  "start": { "line": 0, "character": 11 },
                  "end": { "line": 0, "character": 15 }
               }
            })
        );
    }

    pub fn hover(client_connection: &Connection, uri: Url, line: u32, character: u32) -> Message {
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
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = hover(&client_connection, script.clone(), 2, 0);
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
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = hover(&client_connection, script.clone(), 3, 0);
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
                    "value": "Renders some greeting message\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
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
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = hover(&client_connection, script.clone(), 5, 8);
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
                    "value": "Concatenate multiple strings into a single string, with an optional separator between each.\n### Usage \n```nu\n  str join {flags} <separator?>\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `separator: string` - Optional separator to use when creating string.\n\n\n### Input/output types\n\n```nu\n list<any> | string\n string | string\n\n```\n### Example(s)\n  Create a string from input\n```nu\n  ['nu', 'shell'] | str join\n```\n  Create a string from input with a separator\n```nu\n  ['nu', 'shell'] | str join '-'\n```\n"
                }
            })
        );
    }

    fn complete(client_connection: &Connection, uri: Url, line: u32, character: u32) -> Message {
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
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = complete(&client_connection, script, 2, 9);
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
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = complete(&client_connection, script, 0, 8);
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
    fn complete_command_with_utf8_line() {
        let (client_connection, _recv) =
            initialize_language_server(Some(vec!["utf-8".to_string()]));

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("utf_pipeline.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = complete(&client_connection, script, 0, 14);
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
                        "start": { "line": 0, "character": 9 },
                        "end": { "line": 0, "character": 14 },
                     },
                     "newText": "str trim"
                  },
                  "kind": 3
               }
            ])
        );
    }

    #[test]
    fn complete_command_with_utf16_line() {
        let (client_connection, _recv) =
            initialize_language_server(Some(vec!["utf-16".to_string()]));

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("utf_pipeline.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = complete(&client_connection, script, 0, 13);
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
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let resp = complete(&client_connection, script, 0, 2);
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
