use lsp_textdocument::FullTextDocument;
use nu_parser::parse;
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    ast::find_reference_by_id, path_to_uri, span_to_range, uri_to_path, Id, LanguageServer,
};
use lsp_server::{Message, Request, Response};
use lsp_types::{
    Location, PrepareRenameResponse, ProgressToken, Range, ReferenceParams, RenameParams,
    TextDocumentPositionParams, TextEdit, Uri, WorkspaceEdit, WorkspaceFolder,
};
use miette::{miette, IntoDiagnostic, Result};
use nu_glob::{glob, Paths};
use nu_protocol::{engine::StateWorkingSet, Span};
use serde_json::Value;

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
    /// TODO: WorkDoneProgress -> PartialResults
    pub fn references(&mut self, params: &ReferenceParams) -> Option<Vec<Location>> {
        self.occurrences = BTreeMap::new();
        let mut engine_state = self.new_engine_state();
        let path_uri = params.text_document_position.text_document.uri.to_owned();
        let (mut working_set, id, span, _, _) = self
            .parse_and_find(
                &mut engine_state,
                &path_uri,
                params.text_document_position.position,
            )
            .ok()?;
        self.find_reference_in_workspace(
            &mut working_set,
            &path_uri,
            id,
            span,
            params
                .work_done_progress_params
                .work_done_token
                .to_owned()
                .unwrap_or(ProgressToken::Number(1)),
            "Finding references ...",
        )
        .ok()?;
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

        let (mut working_set, id, span, file_offset, file) =
            self.parse_and_find(&mut engine_state, &path_uri, params.position)?;

        if let Id::Value(_) = id {
            return Err(miette!("\nRename only works for variable/command."));
        }
        if Self::find_definition_span_by_id(&working_set, &id).is_none() {
            return Err(miette!(
                "\nDefinition not found.\nNot allowed to rename built-ins."
            ));
        }
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

        // now continue parsing on other files in the workspace
        self.find_reference_in_workspace(
            &mut working_set,
            &path_uri,
            id,
            span,
            ProgressToken::Number(0),
            "Preparing rename ...",
        )
    }

    fn find_reference_in_workspace(
        &mut self,
        working_set: &mut StateWorkingSet,
        current_uri: &Uri,
        id: Id,
        span: Span,
        token: ProgressToken,
        message: &str,
    ) -> Result<()> {
        let current_workspace_folder = self
            .get_workspace_folder_by_uri(current_uri)
            .ok_or_else(|| miette!("\nCurrent file is not in any workspace"))?;
        let scripts: Vec<PathBuf> = Self::find_nu_scripts_in_folder(&current_workspace_folder.uri)?
            .filter_map(|p| p.ok())
            .collect();
        let len = scripts.len();

        self.send_progress_begin(token.clone(), message)?;
        for (i, fp) in scripts.iter().enumerate() {
            let uri = path_to_uri(fp);
            if let Some(file) = self.docs.get_document(&uri) {
                Self::find_reference_in_file(working_set, file, fp, &id)
            } else {
                let bytes = fs::read(fp).into_diagnostic()?;
                // skip if the file does not contain what we're looking for
                let content_string = String::from_utf8(bytes).into_diagnostic()?;
                let text_to_search =
                    String::from_utf8(working_set.get_span_contents(span).to_vec())
                        .into_diagnostic()?;
                if !content_string.contains(&text_to_search) {
                    continue;
                }
                let temp_file = FullTextDocument::new("nu".to_string(), 0, content_string);
                Self::find_reference_in_file(working_set, &temp_file, fp, &id)
            }
            .and_then(|range| self.occurrences.insert(uri, range));
            self.send_progress_report(token.clone(), (i * 100 / len) as u32, None)?
        }
        self.send_progress_end(token.clone(), Some("Done".to_string()))
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

    fn find_nu_scripts_in_folder(folder_uri: &Uri) -> Result<Paths> {
        let path = uri_to_path(folder_uri);
        if !path.is_dir() {
            return Err(miette!("\nworkspace folder does not exist."));
        }
        let pattern = format!("{}/**/*.nu", path.to_string_lossy());
        glob(&pattern).into_diagnostic()
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
    use crate::tests::{initialize_language_server, open_unchecked};

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
        let messages =
            send_rename_prepare_request(&client_connection, script.clone(), 6, 11, message_num);
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
            assert_json_eq!(
                changes[script.to_string().replace("foo", "bar")],
                serde_json::json!([
                       {
                           "range": { "start": { "line": 5, "character": 4 }, "end": { "line": 5, "character": 11 } },
                           "newText": "new"
                       }
                ])
            );
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

        let message_num = 4;
        let messages =
            send_rename_prepare_request(&client_connection, script.clone(), 3, 5, message_num);
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
}
