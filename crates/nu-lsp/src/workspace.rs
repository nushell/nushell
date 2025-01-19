use lsp_textdocument::FullTextDocument;
use nu_parser::parse;
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    ast::find_reference_by_id, path_to_uri, span_to_range, uri_to_path, Id, LanguageServer,
};
use crossbeam_channel::{Receiver, Sender};
use lsp_server::{Message, Request, Response};
use lsp_types::{
    Location, PrepareRenameResponse, ProgressToken, Range, ReferenceParams, RenameParams,
    TextDocumentPositionParams, TextEdit, Uri, WorkspaceEdit, WorkspaceFolder,
};
use miette::{miette, IntoDiagnostic, Result};
use nu_glob::{glob, Paths};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};
use serde_json::Value;

/// Message type indicating ranges of interest in each doc
#[derive(Debug)]
pub struct RangePerDoc {
    pub uri: Uri,
    pub ranges: Vec<Range>,
}

/// Message sent from background thread to main
#[derive(Debug)]
pub enum InternalMessage {
    RangeMessage(RangePerDoc),
    Cancelled(ProgressToken),
    Finished(ProgressToken),
    OnGoing(ProgressToken, u32),
}

fn find_nu_scripts_in_folder(folder_uri: &Uri) -> Result<Paths> {
    let path = uri_to_path(folder_uri);
    if !path.is_dir() {
        return Err(miette!("\nworkspace folder does not exist."));
    }
    let pattern = format!("{}/**/*.nu", path.to_string_lossy());
    glob(&pattern).into_diagnostic()
}

fn find_reference_in_file(
    working_set: &mut StateWorkingSet,
    file: &FullTextDocument,
    fp: &Path,
    id: &Id,
) -> Option<Vec<Range>> {
    let fp_str = fp.to_str()?;
    let block = parse(
        working_set,
        Some(fp_str),
        file.get_content(None).as_bytes(),
        false,
    );
    let file_span = working_set.get_span_for_filename(fp_str)?;
    let offset = file_span.start;
    let mut references: Vec<Span> = find_reference_by_id(&block, working_set, id);

    // NOTE: for arguments whose declaration is in a signature
    // which is not covered in the AST
    if let Id::Variable(vid) = id {
        let decl_span = working_set.get_variable(*vid).declaration_span;
        if file_span.contains_span(decl_span)
            && decl_span.end > decl_span.start
            && !references.contains(&decl_span)
        {
            references.push(decl_span);
        }
    }
    let occurs: Vec<Range> = references
        .iter()
        .map(|span| span_to_range(span, file, offset))
        .collect();

    // add_block to avoid repeated parsing
    working_set.add_block(block);
    (!occurs.is_empty()).then_some(occurs)
}

impl LanguageServer {
    /// get initial workspace folders from initialization response
    pub fn initialize_workspace_folders(&mut self, init_params: Value) -> Result<()> {
        if let Some(array) = init_params.get("workspaceFolders") {
            let folders: Vec<WorkspaceFolder> =
                serde_json::from_value(array.clone()).into_diagnostic()?;
            for folder in folders {
                self.workspace_folders.insert(folder.name.clone(), folder);
            }
        }
        Ok(())
    }

    /// The rename request only happens after the client received a `PrepareRenameResponse`,
    /// and a new name typed in, could happen before ranges ready for all files in the workspace folder
    pub fn rename(&mut self, params: &RenameParams) -> Option<WorkspaceEdit> {
        let new_name = params.new_name.to_owned();
        // changes in WorkspaceEdit have mutable key
        #[allow(clippy::mutable_key_type)]
        let changes: HashMap<Uri, Vec<TextEdit>> = self
            .occurrences
            .iter()
            .map(|(uri, ranges)| {
                (
                    uri.clone(),
                    ranges
                        .iter()
                        .map(|range| TextEdit {
                            range: *range,
                            new_text: new_name.clone(),
                        })
                        .collect(),
                )
            })
            .collect();
        Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        })
    }

    /// Goto references response
    /// # Arguments
    /// - `timeout`: timeout in milliseconds, when timeout
    ///    1. Respond with all ranges found so far
    ///    2. Cancel the background thread
    pub fn references(&mut self, params: &ReferenceParams, timeout: u128) -> Option<Vec<Location>> {
        self.occurrences = BTreeMap::new();
        let mut engine_state = self.new_engine_state();
        let path_uri = params.text_document_position.text_document.uri.to_owned();
        let (working_set, id, span, _) = self
            .parse_and_find(
                &mut engine_state,
                &path_uri,
                params.text_document_position.position,
            )
            .ok()?;
        // have to clone it again in order to move to another thread
        let mut engine_state = self.new_engine_state();
        engine_state.merge_delta(working_set.render()).ok()?;
        let current_workspace_folder = self.get_workspace_folder_by_uri(&path_uri)?;
        let token = params
            .work_done_progress_params
            .work_done_token
            .to_owned()
            .unwrap_or(ProgressToken::Number(1));
        self.channels = self
            .find_reference_in_workspace(
                engine_state,
                current_workspace_folder,
                id,
                span,
                token.clone(),
                "Finding references ...".to_string(),
            )
            .ok();
        // TODO: WorkDoneProgress -> PartialResults for quicker response
        // currently not enabled by `lsp_types` but hackable in `server_capabilities` json
        let time_start = std::time::Instant::now();
        loop {
            if self.handle_internal_messages().ok()? {
                break;
            }
            if time_start.elapsed().as_millis() > timeout {
                self.send_progress_end(token, Some("Timeout".to_string()))
                    .ok()?;
                self.cancel_background_thread();
                self.channels = None;
                break;
            }
        }
        Some(
            self.occurrences
                .iter()
                .flat_map(|(uri, ranges)| {
                    ranges.iter().map(|range| Location {
                        uri: uri.clone(),
                        range: *range,
                    })
                })
                .collect(),
        )
    }

    /// 1. Parse current file to find the content at the cursor that is suitable for a workspace wide renaming
    /// 2. Parse all nu scripts in the same workspace folder, with the variable/command name in it.
    /// 3. Store the results in `self.occurrences` for later rename quest
    pub fn prepare_rename(&mut self, request: Request) -> Result<()> {
        let params: TextDocumentPositionParams =
            serde_json::from_value(request.params).into_diagnostic()?;
        self.occurrences = BTreeMap::new();

        let mut engine_state = self.new_engine_state();
        let path_uri = params.text_document.uri.to_owned();

        let (working_set, id, span, file_offset) =
            self.parse_and_find(&mut engine_state, &path_uri, params.position)?;

        if let Id::Value(_) = id {
            return Err(miette!("\nRename only works for variable/command."));
        }
        if Self::find_definition_span_by_id(&working_set, &id).is_none() {
            return Err(miette!(
                "\nDefinition not found.\nNot allowed to rename built-ins."
            ));
        }

        let docs = match self.docs.lock() {
            Ok(it) => it,
            Err(err) => return Err(miette!(err.to_string())),
        };
        let file = docs
            .get_document(&path_uri)
            .ok_or_else(|| miette!("\nFailed to get document"))?;
        let range = span_to_range(&span, file, file_offset);
        let response = PrepareRenameResponse::Range(range);
        self.connection
            .sender
            .send(Message::Response(Response {
                id: request.id,
                result: serde_json::to_value(response).ok(),
                error: None,
            }))
            .into_diagnostic()?;

        // have to clone it again in order to move to another thread
        let mut engine_state = self.new_engine_state();
        engine_state
            .merge_delta(working_set.render())
            .into_diagnostic()?;
        let current_workspace_folder = self
            .get_workspace_folder_by_uri(&path_uri)
            .ok_or_else(|| miette!("\nCurrent file is not in any workspace"))?;
        // now continue parsing on other files in the workspace
        self.channels = self
            .find_reference_in_workspace(
                engine_state,
                current_workspace_folder,
                id,
                span,
                ProgressToken::Number(0),
                "Preparing rename ...".to_string(),
            )
            .ok();
        Ok(())
    }

    fn find_reference_in_workspace(
        &self,
        engine_state: EngineState,
        current_workspace_folder: WorkspaceFolder,
        id: Id,
        span: Span,
        token: ProgressToken,
        message: String,
    ) -> Result<(Sender<bool>, Arc<Receiver<InternalMessage>>)> {
        let (data_sender, data_receiver) = crossbeam_channel::unbounded::<InternalMessage>();
        let (cancel_sender, cancel_receiver) = crossbeam_channel::bounded::<bool>(1);
        let engine_state = Arc::new(engine_state);
        let docs = self.docs.clone();
        self.send_progress_begin(token.clone(), message)?;

        std::thread::spawn(move || -> Result<()> {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let scripts: Vec<PathBuf> =
                match find_nu_scripts_in_folder(&current_workspace_folder.uri) {
                    Ok(it) => it,
                    Err(_) => {
                        data_sender
                            .send(InternalMessage::Cancelled(token.clone()))
                            .ok();
                        return Ok(());
                    }
                }
                .filter_map(|p| p.ok())
                .collect();
            let len = scripts.len();

            for (i, fp) in scripts.iter().enumerate() {
                #[cfg(test)]
                std::thread::sleep(std::time::Duration::from_millis(200));
                // cancel the loop on cancellation message from main thread
                if cancel_receiver.try_recv().is_ok() {
                    data_sender
                        .send(InternalMessage::Cancelled(token.clone()))
                        .into_diagnostic()?;
                    return Ok(());
                }
                let percentage = (i * 100 / len) as u32;
                let uri = path_to_uri(fp);
                let docs = match docs.lock() {
                    Ok(it) => it,
                    Err(err) => return Err(miette!(err.to_string())),
                };
                let file = if let Some(file) = docs.get_document(&uri) {
                    file
                } else {
                    let bytes = match fs::read(fp) {
                        Ok(it) => it,
                        Err(_) => {
                            // continue on fs error
                            continue;
                        }
                    };
                    // skip if the file does not contain what we're looking for
                    let content_string = String::from_utf8_lossy(&bytes);
                    let text_to_search =
                        String::from_utf8_lossy(working_set.get_span_contents(span));
                    if !content_string.contains(text_to_search.as_ref()) {
                        // progress without any data
                        data_sender
                            .send(InternalMessage::OnGoing(token.clone(), percentage))
                            .into_diagnostic()?;
                        continue;
                    }
                    &FullTextDocument::new("nu".to_string(), 0, content_string.into())
                };
                let _ = find_reference_in_file(&mut working_set, file, fp, &id).map(|ranges| {
                    data_sender
                        .send(InternalMessage::RangeMessage(RangePerDoc { uri, ranges }))
                        .ok();
                    data_sender
                        .send(InternalMessage::OnGoing(token.clone(), percentage))
                        .ok();
                });
            }
            data_sender
                .send(InternalMessage::Finished(token.clone()))
                .into_diagnostic()?;
            Ok(())
        });
        Ok((cancel_sender, Arc::new(data_receiver)))
    }

    fn get_workspace_folder_by_uri(&self, uri: &Uri) -> Option<WorkspaceFolder> {
        let uri_string = uri.to_string();
        self.workspace_folders
            .iter()
            .find_map(|(_, folder)| {
                uri_string
                    .starts_with(&folder.uri.to_string())
                    .then_some(folder)
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::RenameParams;
    use lsp_types::{
        request, request::Request, InitializeParams, PartialResultParams, Position,
        ReferenceContext, ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
        WorkDoneProgressParams, WorkspaceFolder,
    };
    use nu_test_support::fs::fixtures;

    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, send_hover_request};

    fn send_reference_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
        num: usize,
    ) -> Vec<Message> {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: request::References::METHOD.to_string(),
                params: serde_json::to_value(ReferenceParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    context: ReferenceContext {
                        include_declaration: true,
                    },
                    partial_result_params: PartialResultParams::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();

        (0..num)
            .map(|_| {
                client_connection
                    .receiver
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap()
            })
            .collect()
    }

    fn send_rename_prepare_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
        num: usize,
        immediate_cancellation: bool,
    ) -> Vec<Message> {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: request::PrepareRenameRequest::METHOD.to_string(),
                params: serde_json::to_value(TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position { line, character },
                })
                .unwrap(),
            }))
            .unwrap();

        // use a hover request to interrupt
        if immediate_cancellation {
            send_hover_request(client_connection, uri.clone(), line, character);
        }

        (0..num)
            .map(|_| {
                client_connection
                    .receiver
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap()
            })
            .collect()
    }

    fn send_rename_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: request::Rename::METHOD.to_string(),
                params: serde_json::to_value(RenameParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    new_name: "new".to_string(),
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
    fn command_reference_in_workspace() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        let (client_connection, _recv) = initialize_language_server(Some(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: path_to_uri(&script),
                name: "random name".to_string(),
            }]),
            ..Default::default()
        }));
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let message_num = 5;
        let messages =
            send_reference_request(&client_connection, script.clone(), 0, 12, message_num);
        assert_eq!(messages.len(), message_num);
        for message in messages {
            match message {
                Message::Notification(n) => assert_eq!(n.method, "$/progress"),
                Message::Response(r) => {
                    let result = r.result.unwrap();
                    let array = result.as_array().unwrap();
                    assert!(array.contains(&serde_json::json!(
                            {
                                "uri": script.to_string().replace("foo", "bar"),
                                "range": { "start": { "line": 4, "character": 2 }, "end": { "line": 4, "character": 7 } }
                            }
                        )
                    ));
                    assert!(array.contains(&serde_json::json!(
                            {
                                "uri": script.to_string(),
                                "range": { "start": { "line": 0, "character": 11 }, "end": { "line": 0, "character": 16 } }
                            }
                        )
                    ));
                }
                _ => panic!("unexpected message type"),
            }
        }
    }

    #[test]
    fn quoted_command_reference_in_workspace() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        let (client_connection, _recv) = initialize_language_server(Some(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: path_to_uri(&script),
                name: "random name".to_string(),
            }]),
            ..Default::default()
        }));
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let message_num = 5;
        let messages =
            send_reference_request(&client_connection, script.clone(), 6, 11, message_num);
        assert_eq!(messages.len(), message_num);
        for message in messages {
            match message {
                Message::Notification(n) => assert_eq!(n.method, "$/progress"),
                Message::Response(r) => {
                    let result = r.result.unwrap();
                    let array = result.as_array().unwrap();
                    assert!(array.contains(&serde_json::json!(
                            {
                                "uri": script.to_string().replace("foo", "bar"),
                                "range": { "start": { "line": 5, "character": 4 }, "end": { "line": 5, "character": 11 } }
                            }
                        )
                    ));
                    assert!(array.contains(&serde_json::json!(
                            {
                                "uri": script.to_string(),
                                "range": { "start": { "line": 6, "character": 12 }, "end": { "line": 6, "character": 19 } }
                            }
                        )
                    ));
                }
                _ => panic!("unexpected message type"),
            }
        }
    }

    #[test]
    fn rename_quoted_command() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        let (client_connection, _recv) = initialize_language_server(Some(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: path_to_uri(&script),
                name: "random name".to_string(),
            }]),
            ..Default::default()
        }));
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let message_num = 5;
        let messages = send_rename_prepare_request(
            &client_connection,
            script.clone(),
            6,
            11,
            message_num,
            false,
        );
        assert_eq!(messages.len(), message_num);
        for message in messages {
            match message {
                Message::Notification(n) => assert_eq!(n.method, "$/progress"),
                Message::Response(r) => assert_json_eq!(
                    r.result,
                    serde_json::json!({
                        "start": { "line": 6, "character": 12 },
                        "end": { "line": 6, "character": 19 }
                    }),
                ),
                _ => panic!("unexpected message type"),
            }
        }

        if let Message::Response(r) = send_rename_request(&client_connection, script.clone(), 6, 11)
        {
            let changes = r.result.unwrap()["changes"].clone();
            assert_json_eq!(
                changes[script.to_string()],
                serde_json::json!([
                    {
                        "range": { "start": { "line": 6, "character": 12 }, "end": { "line": 6, "character": 19 } },
                        "newText": "new"
                    }
                ])
            );
            let changs_bar = changes[script.to_string().replace("foo", "bar")]
                .as_array()
                .unwrap();
            assert!(
                changs_bar.contains(
                &serde_json::json!({
                    "range": { "start": { "line": 5, "character": 4 }, "end": { "line": 5, "character": 11 } },
                    "newText": "new"
                })
            ));
            assert!(
                changs_bar.contains(
                &serde_json::json!({
                    "range": { "start": { "line": 0, "character": 20 }, "end": { "line": 0, "character": 27 } },
                    "newText": "new"
                })
            ));
        } else {
            panic!()
        }
    }

    #[test]
    fn rename_command_argument() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        let (client_connection, _recv) = initialize_language_server(Some(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: path_to_uri(&script),
                name: "random name".to_string(),
            }]),
            ..Default::default()
        }));
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let message_num = 5;
        let messages = send_rename_prepare_request(
            &client_connection,
            script.clone(),
            3,
            5,
            message_num,
            false,
        );
        assert_eq!(messages.len(), message_num);
        for message in messages {
            match message {
                Message::Notification(n) => assert_eq!(n.method, "$/progress"),
                Message::Response(r) => assert_json_eq!(
                    r.result,
                    serde_json::json!({
                        "start": { "line": 3, "character": 3 },
                        "end": { "line": 3, "character": 8 }
                    }),
                ),
                _ => panic!("unexpected message type"),
            }
        }

        if let Message::Response(r) = send_rename_request(&client_connection, script.clone(), 3, 5)
        {
            assert_json_eq!(
                r.result,
                serde_json::json!({
                    "changes": {
                        script.to_string(): [
                            {
                                "range": { "start": { "line": 3, "character": 3 }, "end": { "line": 3, "character": 8 } },
                                "newText": "new"
                            },
                            {
                                "range": { "start": { "line": 1, "character": 2 }, "end": { "line": 1, "character": 7 } },
                                "newText": "new"
                            }
                        ]
                    }
                }),
            )
        } else {
            panic!()
        }
    }

    #[test]
    fn rename_cancelled() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        let (client_connection, _recv) = initialize_language_server(Some(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: path_to_uri(&script),
                name: "random name".to_string(),
            }]),
            ..Default::default()
        }));
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let message_num = 3;
        let messages = send_rename_prepare_request(
            &client_connection,
            script.clone(),
            6,
            11,
            message_num,
            true,
        );
        assert_eq!(messages.len(), message_num);
        if let Some(Message::Notification(cancel_notification)) = &messages.last() {
            assert_json_eq!(
                cancel_notification.params["value"],
                serde_json::json!({ "kind": "end", "message": "interrupted." })
            );
        } else {
            panic!("Progress not cancelled");
        };
        for message in messages {
            match message {
                Message::Notification(n) => assert_eq!(n.method, "$/progress"),
                // the response of the preempting hover request
                Message::Response(r) => assert_json_eq!(
                    r.result,
                    serde_json::json!({
                            "contents": {
                            "kind": "markdown",
                            "value": "\n-----\n### Usage \n```nu\n  foo str {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                        }
                    }),
                ),
                _ => panic!("unexpected message type"),
            }
        }

        if let Message::Response(r) = send_rename_request(&client_connection, script.clone(), 6, 11)
        {
            // should not return any changes
            assert_json_eq!(r.result.unwrap()["changes"], serde_json::json!({}));
        } else {
            panic!()
        }
    }
}
