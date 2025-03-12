use std::sync::Arc;

use lsp_textdocument::FullTextDocument;
use lsp_types::{SemanticToken, SemanticTokens, SemanticTokensParams};
use nu_protocol::{
    ast::{Block, Expr, Expression, Traverse},
    engine::StateWorkingSet,
    Span,
};

use crate::{span_to_range, LanguageServer};

/// Important to keep spans in increasing order,
/// since `SemanticToken`s are created by relative positions
/// to one's previous token
///
/// Currently supported types:
/// 1. internal command names with space
fn extract_semantic_tokens_from_expression(
    expr: &Expression,
    working_set: &StateWorkingSet,
) -> Option<Vec<Span>> {
    let closure = |e| extract_semantic_tokens_from_expression(e, working_set);
    match &expr.expr {
        Expr::Call(call) => {
            let command_name_bytes = working_set.get_span_contents(call.head);
            let head_span = if command_name_bytes.contains(&b' ')
                // Some keywords that are already highlighted properly, e.g. by tree-sitter-nu
                && !command_name_bytes.starts_with(b"export")
                && !command_name_bytes.starts_with(b"overlay")
            {
                vec![call.head]
            } else {
                vec![]
            };
            let spans = head_span
                .into_iter()
                .chain(
                    call.arguments
                        .iter()
                        .filter_map(|arg| arg.expr())
                        .flat_map(|e| e.flat_map(working_set, &closure)),
                )
                .collect();
            Some(spans)
        }
        _ => None,
    }
}

impl LanguageServer {
    pub(crate) fn get_semantic_tokens(
        &mut self,
        params: &SemanticTokensParams,
    ) -> Option<SemanticTokens> {
        self.semantic_tokens
            .get(&params.text_document.uri)
            .map(|vec| SemanticTokens {
                result_id: None,
                data: vec.clone(),
            })
    }

    pub(crate) fn extract_semantic_tokens(
        working_set: &StateWorkingSet,
        block: &Arc<Block>,
        offset: usize,
        file: &FullTextDocument,
    ) -> Vec<SemanticToken> {
        let spans = block.flat_map(working_set, &|e| {
            extract_semantic_tokens_from_expression(e, working_set)
        });
        let mut last_token_line = 0;
        let mut last_token_char = 0;
        let mut last_span = Span::unknown();
        let mut tokens = vec![];
        for sp in spans {
            let range = span_to_range(&sp, file, offset);
            // shouldn't happen
            if sp < last_span {
                continue;
            }
            let mut delta_start = range.start.character;
            if range.end.line == last_token_line {
                delta_start -= last_token_char;
            }
            tokens.push(SemanticToken {
                delta_start,
                delta_line: range.end.line.saturating_sub(last_token_line),
                length: range.end.character.saturating_sub(range.start.character),
                // 0 means function in semantic_token_legend
                token_type: 0,
                token_modifiers_bitset: 0,
            });
            last_span = sp;
            last_token_line = range.end.line;
            last_token_char = range.start.character;
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::{
        request::{Request, SemanticTokensFullRequest},
        TextDocumentIdentifier, Uri, WorkDoneProgressParams,
    };
    use lsp_types::{PartialResultParams, SemanticTokensParams};
    use nu_test_support::fs::fixtures;

    fn send_semantic_token_request(client_connection: &Connection, uri: Uri) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: SemanticTokensFullRequest::METHOD.to_string(),
                params: serde_json::to_value(SemanticTokensParams {
                    text_document: TextDocumentIdentifier { uri },
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
    fn semantic_token_internals() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("semantic_tokens");
        script.push("internals.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_semantic_token_request(&client_connection, script.clone());

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!(
            { "data": [
                // delta_line, delta_start, length, token_type, token_modifiers_bitset
                0, 0, 13, 0, 0,
                1, 2, 10, 0, 0,
                7, 15, 13, 0, 0,
                0, 20, 10, 0, 0,
                4, 0, 7, 0, 0
            ]})
        );
    }
}
