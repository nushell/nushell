use crate::ast::{ast_flat_map, expr_flat_map};
use crate::{span_to_range, LanguageServer};
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, InlayHintTooltip, MarkupContent,
    MarkupKind, Position, Range,
};
use nu_protocol::{
    ast::{Argument, Block, Expr, Expression, Operator},
    engine::StateWorkingSet,
    Type,
};
use std::sync::Arc;

fn type_short_name(t: &Type) -> String {
    match t {
        Type::Custom(_) => String::from("custom"),
        Type::Record(_) => String::from("record"),
        Type::Table(_) => String::from("table"),
        Type::List(_) => String::from("list"),
        _ => t.to_string(),
    }
}

fn extract_inlay_hints_from_expression(
    expr: &Expression,
    working_set: &StateWorkingSet,
    offset: &usize,
    file: &FullTextDocument,
) -> Option<Vec<InlayHint>> {
    let closure = |e| extract_inlay_hints_from_expression(e, working_set, offset, file);
    let recur = |expr| expr_flat_map(expr, working_set, &closure);
    match &expr.expr {
        Expr::BinaryOp(lhs, op, rhs) => {
            let mut hints: Vec<InlayHint> =
                [lhs, op, rhs].into_iter().flat_map(|e| recur(e)).collect();
            if let Expr::Operator(Operator::Assignment(_)) = op.expr {
                let position = span_to_range(&lhs.span, file, *offset).end;
                let type_rhs = type_short_name(&rhs.ty);
                let type_lhs = type_short_name(&lhs.ty);
                let type_string = match (type_lhs.as_str(), type_rhs.as_str()) {
                    ("any", _) => type_rhs,
                    (_, "any") => type_lhs,
                    _ => type_lhs,
                };
                hints.push(InlayHint {
                    kind: Some(InlayHintKind::TYPE),
                    label: InlayHintLabel::String(format!(": {}", type_string)),
                    position,
                    text_edits: None,
                    tooltip: None,
                    data: None,
                    padding_left: None,
                    padding_right: None,
                })
            }
            Some(hints)
        }
        Expr::VarDecl(var_id) => {
            let position = span_to_range(&expr.span, file, *offset).end;
            // skip if the type is already specified in code
            if file
                .get_content(Some(Range {
                    start: position,
                    end: Position {
                        line: position.line,
                        character: position.character + 1,
                    },
                }))
                .contains(':')
            {
                return Some(Vec::new());
            }
            let var = working_set.get_variable(*var_id);
            let type_string = type_short_name(&var.ty);
            Some(vec![
                (InlayHint {
                    kind: Some(InlayHintKind::TYPE),
                    label: InlayHintLabel::String(format!(": {}", type_string)),
                    position,
                    text_edits: None,
                    tooltip: None,
                    data: None,
                    padding_left: None,
                    padding_right: None,
                }),
            ])
        }
        Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            // skip those defined outside of the project
            working_set.get_block(decl.block_id()?).span?;
            let signatures = decl.signature();
            let signatures = [
                signatures.required_positional,
                signatures.optional_positional,
            ]
            .concat();
            let arguments = &call.arguments;
            let mut sig_idx = 0;
            let mut hints = Vec::new();
            for arg in arguments {
                match arg {
                    // skip the rest when spread/unknown arguments encountered
                    Argument::Spread(expr) | Argument::Unknown(expr) => {
                        hints.extend(recur(expr));
                        sig_idx = signatures.len();
                        continue;
                    }
                    // skip current for flags
                    Argument::Named((_, _, Some(expr))) => {
                        hints.extend(recur(expr));
                        continue;
                    }
                    Argument::Positional(expr) => {
                        if let Some(sig) = signatures.get(sig_idx) {
                            sig_idx += 1;
                            let position = span_to_range(&arg.span(), file, *offset).start;
                            hints.push(InlayHint {
                                kind: Some(InlayHintKind::PARAMETER),
                                label: InlayHintLabel::String(format!("{}:", sig.name)),
                                position,
                                text_edits: None,
                                tooltip: Some(InlayHintTooltip::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!("`{}: {}`", sig.shape, sig.desc),
                                })),
                                data: None,
                                padding_left: None,
                                padding_right: None,
                            });
                        }
                        hints.extend(recur(expr));
                    }
                    _ => {
                        continue;
                    }
                }
            }
            Some(hints)
        }
        _ => None,
    }
}

impl LanguageServer {
    pub(crate) fn get_inlay_hints(&mut self, params: &InlayHintParams) -> Option<Vec<InlayHint>> {
        Some(self.inlay_hints.get(&params.text_document.uri)?.clone())
    }

    pub(crate) fn extract_inlay_hints(
        working_set: &StateWorkingSet,
        block: &Arc<Block>,
        offset: usize,
        file: &FullTextDocument,
    ) -> Vec<InlayHint> {
        ast_flat_map(block, working_set, &|e| {
            extract_inlay_hints_from_expression(e, working_set, &offset, file)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::{
        request::{InlayHintRequest, Request},
        InlayHintParams, Position, Range, TextDocumentIdentifier, Uri, WorkDoneProgressParams,
    };
    use nu_test_support::fs::fixtures;

    fn send_inlay_hint_request(client_connection: &Connection, uri: Uri) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: InlayHintRequest::METHOD.to_string(),
                params: serde_json::to_value(InlayHintParams {
                    text_document: TextDocumentIdentifier { uri },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    // all inlay hints in the file are returned anyway
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
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
    fn inlay_hint_variable_type() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("type.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_inlay_hint_request(&client_connection, script.clone());

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!([
                { "position": { "line": 0, "character": 9 }, "label": ": int", "kind": 1 },
                { "position": { "line": 1, "character": 7 }, "label": ": string", "kind": 1 },
                { "position": { "line": 2, "character": 8 }, "label": ": bool", "kind": 1 },
                { "position": { "line": 3, "character": 9 }, "label": ": float", "kind": 1 },
                { "position": { "line": 4, "character": 8 }, "label": ": list", "kind": 1 },
                { "position": { "line": 5, "character": 10 }, "label": ": record", "kind": 1 },
                { "position": { "line": 6, "character": 11 }, "label": ": closure", "kind": 1 }
            ])
        );
    }

    #[test]
    fn inlay_hint_assignment_type() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("assignment.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_inlay_hint_request(&client_connection, script.clone());

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!([
                { "position": { "line": 0, "character": 8 }, "label": ": int", "kind": 1 },
                { "position": { "line": 1, "character": 10 }, "label": ": float", "kind": 1 },
                { "position": { "line": 2, "character": 10 }, "label": ": table", "kind": 1 },
                { "position": { "line": 3, "character": 9 }, "label": ": list", "kind": 1 },
                { "position": { "line": 4, "character": 11 }, "label": ": record", "kind": 1 },
                { "position": { "line": 6, "character": 7 }, "label": ": filesize", "kind": 1 },
                { "position": { "line": 7, "character": 7 }, "label": ": filesize", "kind": 1 },
                { "position": { "line": 8, "character": 4 }, "label": ": filesize", "kind": 1 }
            ])
        );
    }

    #[test]
    fn inlay_hint_parameter_names() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("param.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_inlay_hint_request(&client_connection, script.clone());

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!([
                {
                    "position": { "line": 9, "character": 9 },
                    "label": "a1:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                },
                {
                    "position": { "line": 9, "character": 11 },
                    "label": "a2:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                },
                {
                    "position": { "line": 9, "character": 18 },
                    "label": "a3:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: arg3`" }
                },
                {
                    "position": { "line": 10, "character": 6 },
                    "label": "a1:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                },
                {
                    "position": { "line": 11, "character": 2 },
                    "label": "a2:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                },
                {
                    "position": { "line": 12, "character": 11 },
                    "label": "a1:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                },
                {
                    "position": { "line": 12, "character": 13 },
                    "label": "a2:",
                    "kind": 2,
                    "tooltip": { "kind": "markdown", "value": "`any: `" }
                }
            ])
        );
    }
}
