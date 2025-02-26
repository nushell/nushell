use std::sync::Arc;

use crate::{uri_to_path, LanguageServer};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, CompletionTextEdit, Documentation, MarkupContent, MarkupKind, Range,
    TextEdit,
};
use nu_cli::{NuCompleter, SuggestionKind};
use nu_protocol::engine::Stack;

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

        let (results, engine_state) = if need_fallback {
            let engine_state = Arc::new(self.initial_engine_state.clone());
            let completer = NuCompleter::new(engine_state.clone(), Arc::new(Stack::new()));
            (
                completer.fetch_completions_at(&file_text[..location], location),
                engine_state,
            )
        } else {
            let engine_state = Arc::new(self.new_engine_state());
            let completer = NuCompleter::new(engine_state.clone(), Arc::new(Stack::new()));
            let file_path = uri_to_path(&path_uri);
            let filename = file_path.to_str()?;

            let results = completer.fetch_completions_within_file(filename, location, &file_text);
            (results, engine_state)
        };

        (!results.is_empty()).then_some(CompletionResponse::Array(
            results
                .into_iter()
                .map(|r| {
                    let mut start = params.text_document_position.position;
                    start.character -= (r.suggestion.span.end - r.suggestion.span.start) as u32;
                    let decl_id = r.kind.clone().and_then(|kind| {
                        matches!(kind, SuggestionKind::Command(_))
                            .then_some(engine_state.find_decl(r.suggestion.value.as_bytes(), &[])?)
                    });

                    CompletionItem {
                        label: r.suggestion.value.clone(),
                        label_details: r
                            .kind
                            .clone()
                            .map(|kind| match kind {
                                SuggestionKind::Type(t) => t.to_string(),
                                SuggestionKind::Command(cmd) => cmd.to_string(),
                                SuggestionKind::Module => "".to_string(),
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
                nu_protocol::engine::CommandType::External => Some(CompletionItemKind::INTERFACE),
                _ => None,
            },
            SuggestionKind::Module => Some(CompletionItemKind::MODULE),
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
        let (client_connection, _recv) = initialize_language_server(None);

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
                    "labelDetails": { "description": "string" },
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
    fn complete_command_with_space() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_complete_request(&client_connection, script, 0, 8);

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "config nu",
                    "detail": "Edit nu configurations.",
                    "textEdit": { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 8 }, },
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

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "str trim",
                    "labelDetails": { "description": "built-in" },
                    "detail": "Trim whitespace or specific character.",
                    "textEdit": {
                        "range": { "start": { "line": 0, "character": 8 }, "end": { "line": 0, "character": 13 }, },
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

        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!([
                {
                    "label": "overlay",
                    "labelDetails": { "description": "keyword" },
                    "textEdit": {
                        "newText": "overlay",
                        "range": { "start": { "character": 0, "line": 0 }, "end": { "character": 2, "line": 0 } }
                    },
                    "kind": 14
                },
            ])
        );
    }
}
