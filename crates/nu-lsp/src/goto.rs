use crate::{Id, LanguageServer};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse};
use nu_protocol::engine::StateWorkingSet;
use nu_protocol::Span;

impl LanguageServer {
    pub(crate) fn find_definition_span_by_id(
        working_set: &StateWorkingSet,
        id: &Id,
    ) -> Option<Span> {
        match id {
            Id::Declaration(decl_id) => {
                let block_id = working_set.get_decl(*decl_id).block_id()?;
                working_set.get_block(block_id).span
            }
            Id::Variable(var_id) => {
                let var = working_set.get_variable(*var_id);
                Some(var.declaration_span)
            }
            Id::Module(module_id) => {
                let module = working_set.get_module(*module_id);
                module.span
            }
            _ => None,
        }
    }

    pub(crate) fn goto_definition(
        &mut self,
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
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

        Some(GotoDefinitionResponse::Scalar(self.get_location_by_span(
            working_set.files(),
            &Self::find_definition_span_by_id(&working_set, &id)?,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::{assert_json_eq, assert_json_include};
    use lsp_server::{Connection, Message};
    use lsp_types::{
        request::{GotoDefinition, Request},
        GotoDefinitionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };
    use nu_test_support::fs::{fixtures, root};

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
        let (client_connection, _recv) = initialize_language_server(None);

        let mut none_existent_path = root();
        none_existent_path.push("none-existent.nu");
        let script = path_to_uri(&none_existent_path);
        let resp = send_goto_definition_request(&client_connection, script.clone(), 0, 0);

        assert_json_eq!(result_from_message(resp), serde_json::json!(null));
    }

    #[test]
    fn goto_definition_of_variable() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 12);

        assert_json_eq!(
            result_from_message(resp),
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
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 1);

        assert_json_eq!(
            result_from_message(resp),
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
    fn goto_definition_of_command_unicode() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command_unicode.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 2);

        assert_json_eq!(
            result_from_message(resp),
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
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 14);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script,
                "range": {
                "start": { "line": 0, "character": 11 },
                "end": { "line": 0, "character": 15 }
            }
            })
        );
    }

    #[test]
    fn goto_definition_of_variable_in_else_block() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("else.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 21);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 7 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_variable_in_match_guard() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("match.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 9);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 7 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_variable_in_each() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("collect.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 16);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 7 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 3, 15);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": 1, "character": 29 },
                    "end": { "line": 1, "character": 30 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module_in_another_file() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 0, 23);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 1, "character": 29 },
                    "end": { "line": 1, "character": 30 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module_in_hide() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 3, 6);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 1, "character": 29 },
                    "end": { "line": 1, "character": 30 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module_in_overlay() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 20);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 3 }
                }
            })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 25);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 3 }
                }
            })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 30);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 3 }
                }
            })
        );
    }
}
