use std::path::Path;

use crate::{Id, LanguageServer, path_to_uri, span_to_range};
use lsp_textdocument::FullTextDocument;
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};
use nu_protocol::Span;
use nu_protocol::engine::{CachedFile, StateWorkingSet};

impl LanguageServer {
    fn get_location_by_span<'a>(
        &self,
        files: impl Iterator<Item = &'a CachedFile>,
        span: &Span,
    ) -> Option<Location> {
        for cached_file in files.into_iter() {
            if cached_file.covered_span.contains(span.start) {
                let path = Path::new(&*cached_file.name);
                // skip nu-std files
                // TODO: maybe find them in vendor directories?
                if path.is_relative() {
                    let _ = self.send_log_message(
                        lsp_types::MessageType::WARNING,
                        format!(
                            "Location found in file {path:?}, but absolute path is expected. Skipping..."
                        ),
                    );
                    continue;
                }
                let target_uri = path_to_uri(path);
                if let Some(file) = self.docs.lock().ok()?.get_document(&target_uri) {
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, file, cached_file.covered_span.start),
                    });
                } else {
                    if !path.is_file() {
                        return None;
                    }
                    // in case where the document is not opened yet,
                    // typically included by the `use/source` command
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

    pub(crate) fn find_definition_span_by_id(
        working_set: &StateWorkingSet,
        id: &Id,
    ) -> Option<Span> {
        match id {
            Id::Declaration(decl_id) => {
                let block_id = working_set.get_decl(*decl_id).block_id()?;
                working_set.get_block(block_id).span
            }
            Id::Variable(var_id, _) => {
                let var = working_set.get_variable(*var_id);
                Some(var.declaration_span)
            }
            Id::Module(module_id, _) => {
                let module = working_set.get_module(*module_id);
                module.span
            }
            Id::CellPath(var_id, cell_path) => {
                let var = working_set.get_variable(*var_id);
                Some(
                    var.const_val
                        .as_ref()
                        .and_then(|val| val.follow_cell_path(cell_path).ok())
                        .map(|val| val.span())
                        .unwrap_or(var.declaration_span),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn goto_definition(
        &mut self,
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let path_uri = &params.text_document_position_params.text_document.uri;
        let mut engine_state = self.new_engine_state(Some(path_uri));
        let (working_set, id, _, _) = self
            .parse_and_find(
                &mut engine_state,
                path_uri,
                params.text_document_position_params.position,
            )
            .ok()?;

        Some(GotoDefinitionResponse::Scalar(self.get_location_by_span(
            working_set.files(),
            &Self::find_definition_span_by_id(&working_set, &id)?,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message, Notification};
    use lsp_types::{
        GotoDefinitionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
        request::{GotoDefinition, Request},
    };
    use nu_test_support::fs::{fixtures, root};
    use rstest::rstest;

    fn send_goto_definition_request(
        client_connection: &Connection,
        uri: Uri,
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
    fn goto_definition_for_none_existing_file() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut none_existent_path = root();
        none_existent_path.push("none-existent.nu");
        let script = path_to_uri(&none_existent_path);
        let resp = send_goto_definition_request(&client_connection, script, 0, 0);

        assert_json_eq!(result_from_message(resp), serde_json::Value::Null);
    }

    #[rstest]
    #[case::variable("goto/var.nu", (2, 12), None, (0, 4), Some((0, 12)))]
    #[case::command("goto/command.nu", (4, 1), None, (0, 17), Some((2, 1)))]
    #[case::command_unicode("goto/command_unicode.nu", (4, 2), None, (0, 19), Some((2, 1)))]
    #[case::command_parameter("goto/command.nu", (1, 14), None, (0, 11), Some((0, 15)))]
    #[case::variable_in_else_block("goto/else.nu", (1, 21), None, (0, 4), Some((0, 7)))]
    #[case::variable_in_match_guard("goto/match.nu", (2, 9), None, (0, 4), Some((0, 7)))]
    #[case::variable_in_each("goto/collect.nu", (1, 16), None, (0, 4), Some((0, 7)))]
    #[case::module("goto/module.nu", (3, 15), None, (1, 29), Some((1, 30)))]
    #[case::module_in_another_file("goto/use_module.nu", (0, 23), Some("goto/module.nu"), (1, 29), Some((1, 30)))]
    #[case::module_in_hide("goto/use_module.nu", (3, 6), Some("goto/module.nu"), (1, 29), Some((1, 30)))]
    #[case::overlay_first("goto/use_module.nu", (1, 20), Some("goto/module.nu"), (0, 0), None)]
    #[case::overlay_second("goto/use_module.nu", (1, 25), Some("goto/module.nu"), (0, 0), None)]
    #[case::overlay_third("goto/use_module.nu", (2, 30), Some("goto/module.nu"), (0, 0), None)]
    #[case::cell_path_first("hover/use.nu", (2, 7), Some("hover/cell_path.nu"), (1, 10), None)]
    #[case::cell_path_second("hover/use.nu", (2, 9), Some("hover/cell_path.nu"), (1, 17), None)]
    fn goto_definition_single_request(
        #[case] filename: &str,
        #[case] cursor_position: (u32, u32),
        #[case] expected_file: Option<&str>,
        #[case] expected_start: (usize, usize),
        #[case] expected_end: Option<(usize, usize)>,
    ) {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push(filename);
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let (line, character) = cursor_position;
        let resp =
            send_goto_definition_request(&client_connection, script.clone(), line, character);
        let result = result_from_message(resp);

        let mut target_uri = script.to_string();
        if let Some(name) = expected_file {
            target_uri = target_uri.replace(filename, name);
        }
        assert_json_eq!(result["uri"], serde_json::json!(target_uri));
        let (line, character) = expected_start;
        assert_json_eq!(
            result["range"]["start"],
            serde_json::json!({ "line": line, "character": character })
        );
        if let Some((line, character)) = expected_end {
            assert_json_eq!(
                result["range"]["end"],
                serde_json::json!({ "line": line, "character": character })
            );
        }
    }

    #[test]
    // https://github.com/nushell/nushell/issues/16539
    fn goto_definition_in_new_file() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp/no_such_file.nu");
        let script = path_to_uri(&script);

        let file_content = r#"def foo [] {}; foo"#;
        let _ = open(
            &client_connection,
            script.clone(),
            Some(file_content.into()),
        );
        let resp = send_goto_definition_request(
            &client_connection,
            script.clone(),
            0,
            file_content.len() as u32 - 1,
        );
        let result = result_from_message(resp);

        assert_json_eq!(
            result,
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": 0, "character": 11 },
                    "end": { "line": 0, "character": 13 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_on_stdlib_should_not_panic() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp/goto/use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script, 7, 19);
        match resp {
            Message::Notification(Notification { params, .. }) => {
                assert!(
                    params["message"]
                        .to_string()
                        .contains("absolute path is expected")
                );
            }
            _ => panic!("Unexpected message!"),
        }
    }
}
