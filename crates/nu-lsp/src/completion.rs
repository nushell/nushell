use std::sync::Arc;

use crate::{span_to_range, uri_to_path, LanguageServer};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTextEdit, Documentation, MarkupContent, MarkupKind, TextEdit,
};
use nu_cli::{NuCompleter, SuggestionKind};
use nu_protocol::{
    engine::{CommandType, Stack},
    Span,
};

impl LanguageServer {
    pub(crate) fn complete(&mut self, params: &CompletionParams) -> Option<CompletionResponse> {
        let path_uri = params.text_document_position.text_document.uri.to_owned();
        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(&path_uri)?;
        let location = file.offset_at(params.text_document_position.position) as usize;
        let file_text = file.get_content(None).to_owned();
        drop(docs);
        // fallback to default completer where
        // the text is truncated to `location` and
        // an extra placeholder token is inserted for correct parsing
        let need_fallback = location == 0
            || file_text
                .get(location - 1..location)
                .and_then(|s| s.chars().next())
                .is_some_and(|c| c.is_whitespace() || "|(){}[]<>,:;".contains(c));

        self.need_parse |= need_fallback;
        let engine_state = Arc::new(self.new_engine_state());
        let completer = NuCompleter::new(engine_state.clone(), Arc::new(Stack::new()));
        let results = if need_fallback {
            completer.fetch_completions_at(&file_text[..location], location)
        } else {
            let file_path = uri_to_path(&path_uri);
            let filename = file_path.to_str()?;
            completer.fetch_completions_within_file(filename, location, &file_text)
        };

        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(&path_uri)?;
        (!results.is_empty()).then_some(CompletionResponse::Array(
            results
                .into_iter()
                .map(|r| {
                    let decl_id = r.kind.clone().and_then(|kind| {
                        matches!(kind, SuggestionKind::Command(_))
                            .then_some(engine_state.find_decl(r.suggestion.value.as_bytes(), &[])?)
                    });

                    let mut label_value = r.suggestion.value;
                    if r.suggestion.append_whitespace {
                        label_value.push(' ');
                    }

                    let span = r.suggestion.span;
                    let text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                        range: span_to_range(&Span::new(span.start, span.end), file, 0),
                        new_text: label_value.clone(),
                    }));

                    CompletionItem {
                        label: label_value,
                        label_details: r
                            .kind
                            .clone()
                            .map(|kind| match kind {
                                SuggestionKind::Value(t) => t.to_string(),
                                SuggestionKind::Command(cmd) => cmd.to_string(),
                                SuggestionKind::Module => "module".to_string(),
                                SuggestionKind::Operator => "operator".to_string(),
                                SuggestionKind::Variable => "variable".to_string(),
                                SuggestionKind::Flag => "flag".to_string(),
                                _ => String::new(),
                            })
                            .map(|s| CompletionItemLabelDetails {
                                detail: None,
                                description: Some(s),
                            }),
                        detail: r.suggestion.description,
                        documentation: r
                            .suggestion
                            .extra
                            .map(|ex| ex.join("\n"))
                            .or(decl_id.map(|decl_id| {
                                Self::get_decl_description(engine_state.get_decl(decl_id), true)
                            }))
                            .map(|value| {
                                Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value,
                                })
                            }),
                        kind: Self::lsp_completion_item_kind(r.kind),
                        text_edit,
                        ..Default::default()
                    }
                })
                .collect(),
        ))
    }

    fn lsp_completion_item_kind(
        suggestion_kind: Option<SuggestionKind>,
    ) -> Option<CompletionItemKind> {
        suggestion_kind.and_then(|suggestion_kind| match suggestion_kind {
            SuggestionKind::Value(t) => match t {
                nu_protocol::Type::String => Some(CompletionItemKind::VALUE),
                _ => None,
            },
            SuggestionKind::CellPath => Some(CompletionItemKind::PROPERTY),
            SuggestionKind::Command(c) => match c {
                CommandType::Keyword => Some(CompletionItemKind::KEYWORD),
                CommandType::Builtin => Some(CompletionItemKind::FUNCTION),
                CommandType::Alias => Some(CompletionItemKind::REFERENCE),
                CommandType::External => Some(CompletionItemKind::INTERFACE),
                CommandType::Custom | CommandType::Plugin => Some(CompletionItemKind::METHOD),
            },
            SuggestionKind::Directory => Some(CompletionItemKind::FOLDER),
            SuggestionKind::File => Some(CompletionItemKind::FILE),
            SuggestionKind::Flag => Some(CompletionItemKind::FIELD),
            SuggestionKind::Module => Some(CompletionItemKind::MODULE),
            SuggestionKind::Operator => Some(CompletionItemKind::OPERATOR),
            SuggestionKind::Variable => Some(CompletionItemKind::VARIABLE),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_include;
    use lsp_server::{Connection, Message};
    use lsp_types::{
        request::{Completion, Request},
        CompletionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };
    use nu_test_support::fs::fixtures;

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
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script, 2, 9);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "$greeting",
                    "labelDetails": { "description": "variable" },
                    "textEdit": {
                        "newText": "$greeting",
                        "range": { "start": { "character": 5, "line": 2 }, "end": { "character": 9, "line": 2 } }
                    },
                    "kind": 6
                }
            ])
        );
    }

    #[test]
    fn complete_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script.clone(), 0, 6);

        #[cfg(not(windows))]
        let detail_str = "detail";
        #[cfg(windows)]
        let detail_str = "detail\r";
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                // defined after the cursor
                { "label": "config n foo bar ", "detail": detail_str, "kind": 2 },
                {
                    "label": "config nu ",
                    "detail": "Edit nu configurations.",
                    "textEdit": { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 8 }, },
                        "newText": "config nu "
                    },
                },
            ])
        );

        // short flag
        let resp = send_complete_request(&client_connection, script.clone(), 1, 18);
        assert!(result_from_message(resp).as_array().unwrap().contains(
            &serde_json::json!({
                "label": "-s ",
                "detail": "test flag",
                "labelDetails": { "description": "flag" },
                "textEdit": { "range": { "start": { "line": 1, "character": 17 }, "end": { "line": 1, "character": 18 }, },
                    "newText": "-s "
                },
                "kind": 5
            })
        ));

        // long flag
        let resp = send_complete_request(&client_connection, script.clone(), 2, 22);
        assert!(result_from_message(resp).as_array().unwrap().contains(
            &serde_json::json!({
                "label": "--long ",
                "detail": "test flag",
                "labelDetails": { "description": "flag" },
                "textEdit": { "range": { "start": { "line": 2, "character": 19 }, "end": { "line": 2, "character": 22 }, },
                    "newText": "--long "
                },
                "kind": 5
            })
        ));

        // file path
        let resp = send_complete_request(&client_connection, script.clone(), 2, 18);
        assert!(result_from_message(resp).as_array().unwrap().contains(
            &serde_json::json!({
                "label": "LICENSE",
                "labelDetails": { "description": "" },
                "textEdit": { "range": { "start": { "line": 2, "character": 17 }, "end": { "line": 2, "character": 18 }, },
                    "newText": "LICENSE"
                },
                "kind": 17
            })
        ));

        // inside parenthesis
        let resp = send_complete_request(&client_connection, script, 10, 34);
        assert!(result_from_message(resp).as_array().unwrap().contains(
            &serde_json::json!({
                "label": "-g ",
                "detail": "count indexes and split using grapheme clusters (all visible chars have length 1)",
                "labelDetails": { "description": "flag" },
                "textEdit": { "range": { "start": { "line": 10, "character": 33 }, "end": { "line": 10, "character": 34 }, },
                    "newText": "-g "
                },
                "kind": 5
            })
        ));
    }

    #[test]
    fn fallback_completion() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("fallback.nu");
        let script = path_to_uri(&script);
        open_unchecked(&client_connection, script.clone());

        // at the very beginning of a file
        let resp = send_complete_request(&client_connection, script.clone(), 0, 0);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "alias ",
                    "labelDetails": { "description": "keyword" },
                    "detail": "Alias a command (with optional flags) to a new name.",
                    "textEdit": {
                        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 }, },
                        "newText": "alias "
                    },
                    "kind": 14
                }
            ])
        );
        // after a white space character
        let resp = send_complete_request(&client_connection, script.clone(), 3, 2);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "alias ",
                    "labelDetails": { "description": "keyword" },
                    "detail": "Alias a command (with optional flags) to a new name.",
                    "textEdit": {
                        "range": { "start": { "line": 3, "character": 2 }, "end": { "line": 3, "character": 2 }, },
                        "newText": "alias "
                    },
                    "kind": 14
                }
            ])
        );
        // fallback file path completion
        let resp = send_complete_request(&client_connection, script, 5, 4);
        assert!(result_from_message(resp).as_array().unwrap().contains(
            &serde_json::json!({
                "label": "LICENSE",
                "labelDetails": { "description": "" },
                "textEdit": { "range": { "start": { "line": 5, "character": 3 }, "end": { "line": 5, "character": 4 }, },
                    "newText": "LICENSE"
                },
                "kind": 17
            })
        ));
    }

    #[test]
    fn complete_command_with_line() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("utf_pipeline.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script, 0, 13);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "str trim ",
                    "labelDetails": { "description": "built-in" },
                    "detail": "Trim whitespace or specific character.",
                    "textEdit": {
                        "range": { "start": { "line": 0, "character": 8 }, "end": { "line": 0, "character": 13 }, },
                        "newText": "str trim "
                    },
                    "kind": 3
                }
            ])
        );
    }

    #[test]
    fn complete_keyword() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("keyword.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script, 0, 2);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "overlay ",
                    "labelDetails": { "description": "keyword" },
                    "textEdit": {
                        "newText": "overlay ",
                        "range": { "start": { "character": 0, "line": 0 }, "end": { "character": 2, "line": 0 } }
                    },
                    "kind": 14
                },
            ])
        );
    }

    #[test]
    fn complete_cell_path() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("cell_path.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script.clone(), 1, 5);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "1",
                    "detail": "string",
                    "textEdit": {
                        "newText": "1",
                        "range": { "start": { "line": 1, "character": 5 }, "end": { "line": 1, "character": 5 } }
                    },
                    "kind": 10
                },
            ])
        );

        let resp = send_complete_request(&client_connection, script, 1, 10);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "baz",
                    "detail": "int",
                    "textEdit": {
                        "newText": "baz",
                        "range": { "start": { "line": 1, "character": 10 }, "end": { "line": 1, "character": 10 } }
                    },
                    "kind": 10
                },
            ])
        );
    }

    #[test]
    fn complete_with_external_completer() {
        let config = "$env.config.completions.external.completer = {|spans| ['--background']}";
        let (client_connection, _recv) = initialize_language_server(Some(config), None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("external.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script.clone(), 0, 11);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "--background",
                    "labelDetails": { "description": "string" },
                    "textEdit": {
                        "newText": "--background",
                        "range": { "start": { "line": 0, "character": 5 }, "end": { "line": 0, "character": 11 } }
                    },
                },
            ])
        );

        // fallback completer, special argument treatment for `sudo`/`doas`
        let resp = send_complete_request(&client_connection, script, 0, 5);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "alias ",
                    "labelDetails": { "description": "keyword" },
                    "detail": "Alias a command (with optional flags) to a new name.",
                    "textEdit": {
                        "range": { "start": { "line": 0, "character": 5 }, "end": { "line": 0, "character": 5 }, },
                        "newText": "alias "
                    },
                    "kind": 14
                },
            ])
        );
    }

    #[test]
    fn complete_operators() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("fallback.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        // fallback completer
        let resp = send_complete_request(&client_connection, script.clone(), 7, 10);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "!= ",
                    "labelDetails": { "description": "operator" },
                    "textEdit": {
                        "newText": "!= ",
                        "range": { "start": { "character": 10, "line": 7 }, "end": { "character": 10, "line": 7 } }
                    },
                    "kind": 24 // operator kind
                }
            ])
        );

        let resp = send_complete_request(&client_connection, script.clone(), 7, 15);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "not-has ",
                    "labelDetails": { "description": "operator" },
                    "textEdit": {
                        "newText": "not-has ",
                        "range": { "start": { "character": 10, "line": 7 }, "end": { "character": 15, "line": 7 } }
                    },
                    "kind": 24 // operator kind
                }
            ])
        );
    }

    #[test]
    fn complete_use_arguments() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("use.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script.clone(), 4, 17);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "std-rfc",
                    "labelDetails": { "description": "module" },
                    "textEdit": {
                        "newText": "std-rfc",
                        "range": { "start": { "character": 11, "line": 4 }, "end": { "character": 17, "line": 4 } }
                    },
                    "kind": 9 // module kind
                }
            ])
        );

        let resp = send_complete_request(&client_connection, script.clone(), 5, 22);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "clip",
                    "labelDetails": { "description": "module" },
                    "textEdit": {
                        "newText": "clip",
                        "range": { "start": { "character": 19, "line": 5 }, "end": { "character": 23, "line": 5 } }
                    },
                    "kind": 9 // module kind
                }
            ])
        );

        let resp = send_complete_request(&client_connection, script.clone(), 5, 35);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "paste",
                    "labelDetails": { "description": "custom" },
                    "textEdit": {
                        "newText": "paste",
                        "range": { "start": { "character": 32, "line": 5 }, "end": { "character": 37, "line": 5 } }
                    },
                    "kind": 2
                }
            ])
        );

        let resp = send_complete_request(&client_connection, script.clone(), 6, 14);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "null_device",
                    "labelDetails": { "description": "variable" },
                    "textEdit": {
                        "newText": "null_device",
                        "range": { "start": { "character": 8, "line": 6 }, "end": { "character": 14, "line": 6 } }
                    },
                    "kind": 6 // variable kind
                }
            ])
        );

        let resp = send_complete_request(&client_connection, script, 7, 13);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "foo",
                    "labelDetails": { "description": "variable" },
                    "textEdit": {
                        "newText": "foo",
                        "range": { "start": { "character": 11, "line": 7 }, "end": { "character": 14, "line": 7 } }
                    },
                    "kind": 6 // variable kind
                }
            ])
        );
    }
}
