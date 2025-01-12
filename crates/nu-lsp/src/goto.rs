use crate::ast::find_id;
use crate::{Id, LanguageServer};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse};

impl LanguageServer {
    pub fn goto_definition(
        &mut self,
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
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

        let span = match id {
            Id::Declaration(decl_id) => {
                let block_id = working_set.get_decl(decl_id).block_id()?;
                working_set.get_block(block_id).span
            }
            Id::Variable(var_id) => {
                let var = working_set.get_variable(var_id);
                Some(var.declaration_span)
            }
            Id::Module(module_id) => {
                let module = working_set.get_module(module_id);
                module.span
            }
            _ => None,
        }?;
        Some(GotoDefinitionResponse::Scalar(
            self.get_location_by_span(working_set.files(), &span)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::request::{GotoDefinition, Request};
    use lsp_types::{
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
        let (client_connection, _recv) = initialize_language_server();

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
                            uri: path_to_uri(&none_existent_path),
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

    #[test]
    fn goto_definition_of_variable() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 12);
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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 1);
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
    fn goto_definition_of_command_unicode() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command_unicode.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 2);
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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 14);
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

    #[test]
    fn goto_definition_of_variable_in_else_block() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("else.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 21);
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
                    "end": { "line": 0, "character": 7 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_variable_in_match_guard() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("match.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 9);
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
                    "end": { "line": 0, "character": 7 }
                }
            })
        );
    }
}
