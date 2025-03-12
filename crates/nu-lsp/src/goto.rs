use std::path::Path;

use crate::{path_to_uri, span_to_range, Id, LanguageServer};
use lsp_textdocument::FullTextDocument;
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};
use nu_protocol::engine::{CachedFile, StateWorkingSet};
use nu_protocol::Span;

impl LanguageServer {
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
            Id::Variable(var_id) => {
                let var = working_set.get_variable(*var_id);
                Some(var.declaration_span)
            }
            Id::Module(module_id) => {
                let module = working_set.get_module(*module_id);
                module.span
            }
            Id::CellPath(var_id, cell_path) => {
                let var = working_set.get_variable(*var_id);
                Some(
                    var.const_val
                        .clone()
                        .and_then(|val| val.follow_cell_path(cell_path, false).ok())
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
    use assert_json_diff::assert_json_eq;
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
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut none_existent_path = root();
        none_existent_path.push("none-existent.nu");
        let script = path_to_uri(&none_existent_path);
        let resp = send_goto_definition_request(&client_connection, script.clone(), 0, 0);

        assert_json_eq!(result_from_message(resp), serde_json::json!(null));
    }

    #[test]
    fn goto_definition_of_variable() {
        let (client_connection, _recv) = initialize_language_server(None, None);

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
    fn goto_definition_of_cell_path() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("cell_path.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 7);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 1, "character": 10 })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 4, 9);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 1, "character": 17 })
        );
    }

    #[test]
    fn goto_definition_of_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

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
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 20);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 25);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 30);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );
    }
}
