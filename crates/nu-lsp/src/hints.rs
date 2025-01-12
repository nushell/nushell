use std::sync::Arc;

use crate::{span_to_range, LanguageServer};
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, InlayHintTooltip, MarkupContent,
    MarkupKind, Position, Range,
};
use nu_protocol::{
    ast::{
        Argument, Block, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Operator,
        Pattern, PipelineRedirection, RecordItem,
    },
    engine::StateWorkingSet,
    Type,
};

/// similar to flatten_block, but allows extra map function
fn ast_flat_map<T, E>(
    ast: &Arc<Block>,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    ast.pipelines
        .iter()
        .flat_map(|pipeline| {
            pipeline.elements.iter().flat_map(|element| {
                expr_flat_map(&element.expr, working_set, extra_args, f_special)
                    .into_iter()
                    .chain(
                        element
                            .redirection
                            .as_ref()
                            .map(|redir| {
                                redirect_flat_map(redir, working_set, extra_args, f_special)
                            })
                            .unwrap_or_default(),
                    )
            })
        })
        .collect()
}

/// generic function that do flat_map on an expression
/// concats all recursive results on sub-expressions
///
/// # Arguments
/// * `f_special` - function that overrides the default behavior
fn expr_flat_map<T, E>(
    expr: &Expression,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    // behavior overridden by f_special
    if let Some(vec) = f_special(expr, working_set, extra_args) {
        return vec;
    }
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    match &expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(block_id.to_owned());
            ast_flat_map(block, working_set, extra_args, f_special)
        }
        Expr::Range(range) => [&range.from, &range.next, &range.to]
            .iter()
            .filter_map(|e| e.as_ref())
            .flat_map(recur)
            .collect(),
        Expr::Call(call) => call
            .arguments
            .iter()
            .filter_map(|arg| arg.expr())
            .flat_map(recur)
            .collect(),
        Expr::ExternalCall(head, args) => recur(head)
            .into_iter()
            .chain(args.iter().flat_map(|arg| match arg {
                ExternalArgument::Regular(e) | ExternalArgument::Spread(e) => recur(e),
            }))
            .collect(),
        Expr::UnaryNot(expr) | Expr::Collect(_, expr) => recur(expr),
        Expr::BinaryOp(lhs, op, rhs) => recur(lhs)
            .into_iter()
            .chain(recur(op))
            .chain(recur(rhs))
            .collect(),
        Expr::MatchBlock(matches) => matches
            .iter()
            .flat_map(|(pattern, expr)| {
                match_pattern_flat_map(pattern, working_set, extra_args, f_special)
                    .into_iter()
                    .chain(recur(expr))
            })
            .collect(),
        Expr::List(items) => items
            .iter()
            .flat_map(|item| match item {
                ListItem::Item(expr) | ListItem::Spread(_, expr) => recur(expr),
            })
            .collect(),
        Expr::Record(items) => items
            .iter()
            .flat_map(|item| match item {
                RecordItem::Spread(_, expr) => recur(expr),
                RecordItem::Pair(key, val) => [key, val].into_iter().flat_map(recur).collect(),
            })
            .collect(),
        Expr::Table(table) => table
            .columns
            .iter()
            .flat_map(recur)
            .chain(table.rows.iter().flat_map(|row| row.iter().flat_map(recur)))
            .collect(),
        Expr::ValueWithUnit(vu) => recur(&vu.expr),
        Expr::FullCellPath(fcp) => recur(&fcp.head),
        Expr::Keyword(kw) => recur(&kw.expr),
        Expr::StringInterpolation(vec) | Expr::GlobInterpolation(vec, _) => {
            vec.iter().flat_map(recur).collect()
        }

        _ => Vec::new(),
    }
}

/// flat_map on match patterns
fn match_pattern_flat_map<T, E>(
    pattern: &MatchPattern,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    let recur_match = |p| match_pattern_flat_map(p, working_set, extra_args, f_special);
    match &pattern.pattern {
        Pattern::Expression(expr) => recur(expr),
        Pattern::List(patterns) | Pattern::Or(patterns) => {
            patterns.iter().flat_map(recur_match).collect()
        }
        Pattern::Record(entries) => entries.iter().flat_map(|(_, p)| recur_match(p)).collect(),
        _ => Vec::new(),
    }
    .into_iter()
    .chain(pattern.guard.as_ref().map(|g| recur(g)).unwrap_or_default())
    .collect()
}

/// flat_map on redirections
fn redirect_flat_map<T, E>(
    redir: &PipelineRedirection,
    working_set: &StateWorkingSet,
    extra_args: &E,
    f_special: fn(&Expression, &StateWorkingSet, &E) -> Option<Vec<T>>,
) -> Vec<T> {
    let recur = |expr| expr_flat_map(expr, working_set, extra_args, f_special);
    match redir {
        PipelineRedirection::Single { target, .. } => target.expr().map(recur).unwrap_or_default(),
        PipelineRedirection::Separate { out, err } => [out, err]
            .iter()
            .filter_map(|t| t.expr())
            .flat_map(recur)
            .collect(),
    }
}

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
    extra_args: &(usize, &FullTextDocument),
) -> Option<Vec<InlayHint>> {
    let span = expr.span;
    let (offset, file) = extra_args;
    let recur = |expr| {
        expr_flat_map(
            expr,
            working_set,
            extra_args,
            extract_inlay_hints_from_expression,
        )
    };
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
            let position = span_to_range(&span, file, *offset).end;
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
    pub fn get_inlay_hints(&mut self, params: &InlayHintParams) -> Option<Vec<InlayHint>> {
        Some(self.inlay_hints.get(&params.text_document.uri)?.clone())
    }

    pub fn extract_inlay_hints(
        &self,
        working_set: &StateWorkingSet,
        block: &Arc<Block>,
        offset: usize,
        file: &FullTextDocument,
    ) -> Vec<InlayHint> {
        ast_flat_map(
            block,
            working_set,
            &(offset, file),
            extract_inlay_hints_from_expression,
        )
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use lsp_types::request::Request;
    use nu_test_support::fs::fixtures;

    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked};
    use lsp_server::{Connection, Message};
    use lsp_types::{
        request::InlayHintRequest, InlayHintParams, Position, Range, TextDocumentIdentifier, Uri,
        WorkDoneProgressParams,
    };

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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("type.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_inlay_hint_request(&client_connection, script.clone());
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("assignment.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_inlay_hint_request(&client_connection, script.clone());
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("param.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_inlay_hint_request(&client_connection, script.clone());
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
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
