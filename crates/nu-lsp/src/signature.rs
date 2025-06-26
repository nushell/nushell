use lsp_types::{
    Documentation, MarkupContent, MarkupKind, ParameterInformation, SignatureHelp,
    SignatureHelpParams, SignatureInformation,
};
use nu_protocol::{
    Flag, PositionalArg, Signature, SyntaxShape, Value,
    ast::{Argument, Call, Expr, Expression, FindMapResult, Traverse},
    engine::StateWorkingSet,
};

use crate::{LanguageServer, uri_to_path};

fn find_active_internal_call<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> FindMapResult<&'a Call> {
    if !expr.span.contains(pos) {
        return FindMapResult::Stop;
    }
    let closure = |e| find_active_internal_call(e, working_set, pos);
    match &expr.expr {
        Expr::Call(call) => {
            if call.head.contains(pos) {
                return FindMapResult::Stop;
            }
            call.arguments
                .iter()
                .find_map(|arg| arg.expr().and_then(|e| e.find_map(working_set, &closure)))
                .or(Some(call.as_ref()))
                .map(FindMapResult::Found)
                .unwrap_or_default()
        }
        _ => FindMapResult::Continue,
    }
}

pub(crate) fn display_flag(flag: &Flag, verbitam: bool) -> String {
    let md_backtick = if verbitam { "`" } else { "" };
    let mut text = String::new();
    if let Some(short_flag) = flag.short {
        text.push_str(&format!("{md_backtick}-{short_flag}{md_backtick}"));
    }
    if !flag.long.is_empty() {
        if flag.short.is_some() {
            text.push_str(", ");
        }
        text.push_str(&format!("{md_backtick}--{}{md_backtick}", flag.long));
    }
    text
}

pub(crate) fn doc_for_arg(
    syntax_shape: Option<SyntaxShape>,
    desc: String,
    default_value: Option<Value>,
    optional: bool,
) -> String {
    let mut text = String::new();
    if let Some(mut shape) = syntax_shape {
        if let SyntaxShape::Keyword(_, inner_shape) = shape {
            shape = *inner_shape;
        }
        text.push_str(&format!(": `<{shape}>`"));
    }
    if !(desc.is_empty() && default_value.is_none()) || optional {
        text.push_str(" -")
    };
    if !desc.is_empty() {
        text.push_str(&format!(" {desc}"));
    };
    if let Some(value) = default_value.as_ref().and_then(|v| v.coerce_str().ok()) {
        text.push_str(&format!(
            " ({}default: `{value}`)",
            if optional { "optional, " } else { "" }
        ));
    } else if optional {
        text.push_str(" (optional)");
    }
    text
}

pub(crate) fn get_signature_label(signature: &Signature, indent: bool) -> String {
    let expand_keyword = |arg: &PositionalArg, optional: bool| match &arg.shape {
        SyntaxShape::Keyword(kwd, _) => {
            format!("{} <{}>", String::from_utf8_lossy(kwd), arg.name)
        }
        _ => {
            if optional {
                arg.name.clone()
            } else {
                format!("<{}>", arg.name)
            }
        }
    };
    let mut label = String::new();
    if indent {
        label.push_str("  ");
    }
    label.push_str(&signature.name);
    if !signature.named.is_empty() {
        label.push_str(" {flags}");
    }
    for required_arg in &signature.required_positional {
        label.push_str(&format!(" {}", expand_keyword(required_arg, false)));
    }
    for optional_arg in &signature.optional_positional {
        label.push_str(&format!(" ({})", expand_keyword(optional_arg, true)));
    }
    if let Some(arg) = &signature.rest_positional {
        label.push_str(&format!(" ...({})", arg.name));
    }
    label
}

impl LanguageServer {
    pub(crate) fn get_signature_help(
        &mut self,
        params: &SignatureHelpParams,
    ) -> Option<SignatureHelp> {
        let path_uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_owned();
        let docs = self.docs.lock().ok()?;
        let file = docs.get_document(&path_uri)?;
        let location = file.offset_at(params.text_document_position_params.position) as usize;
        let file_text = file.get_content(None).to_owned();
        drop(docs);

        let engine_state = self.new_engine_state(Some(&path_uri));
        let mut working_set = StateWorkingSet::new(&engine_state);

        // NOTE: in case the cursor is at the end of the call expression
        let need_placeholder = location == 0
            || file_text
                .get(location - 1..location)
                .is_some_and(|s| s.chars().all(|c| c.is_whitespace()));
        let file_path = uri_to_path(&path_uri);
        let filename = if need_placeholder {
            "lsp_signature_helper_temp_file"
        } else {
            file_path.to_str()?
        };

        let block = if need_placeholder {
            nu_parser::parse(
                &mut working_set,
                Some(filename),
                format!(
                    "{}a{}",
                    file_text.get(..location).unwrap_or_default(),
                    file_text.get(location..).unwrap_or_default()
                )
                .as_bytes(),
                false,
            )
        } else {
            nu_parser::parse(
                &mut working_set,
                Some(filename),
                file_text.as_bytes(),
                false,
            )
        };
        let span = working_set.get_span_for_filename(filename)?;

        let pos_to_search = location.saturating_add(span.start).saturating_sub(1);
        let active_call = block.find_map(&working_set, &|expr: &Expression| {
            find_active_internal_call(expr, &working_set, pos_to_search)
        })?;
        let active_signature = working_set.get_decl(active_call.decl_id).signature();
        let label = get_signature_label(&active_signature, false);

        let mut param_num_before_pos = 0;
        for arg in active_call.arguments.iter() {
            // skip flags
            if matches!(arg, Argument::Named(_)) {
                continue;
            }
            if arg.span().end <= pos_to_search {
                param_num_before_pos += 1;
            } else {
                break;
            }
        }

        let str_to_doc = |s: String| {
            Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: s,
            }))
        };
        let arg_to_param_info = |arg: PositionalArg, optional: bool| ParameterInformation {
            label: lsp_types::ParameterLabel::Simple(arg.name),
            documentation: str_to_doc(doc_for_arg(
                Some(arg.shape),
                arg.desc,
                arg.default_value,
                optional,
            )),
        };
        let flag_to_param_info = |flag: Flag| ParameterInformation {
            label: lsp_types::ParameterLabel::Simple(display_flag(&flag, false)),
            documentation: str_to_doc(doc_for_arg(flag.arg, flag.desc, flag.default_value, false)),
        };

        // positional args
        let mut parameters: Vec<ParameterInformation> = active_signature
            .required_positional
            .into_iter()
            .map(|arg| arg_to_param_info(arg, false))
            .chain(
                active_signature
                    .optional_positional
                    .into_iter()
                    .map(|arg| arg_to_param_info(arg, true)),
            )
            .collect();
        if let Some(rest_arg) = active_signature.rest_positional {
            parameters.push(arg_to_param_info(rest_arg, false));
        }

        let max_idx = parameters.len().saturating_sub(1) as u32;
        let active_parameter = Some(param_num_before_pos.min(max_idx));
        // also include flags in the end, just for documentation
        parameters.extend(active_signature.named.into_iter().map(flag_to_param_info));

        Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label,
                documentation: str_to_doc(active_signature.description),
                parameters: Some(parameters),
                active_parameter,
            }],
            active_signature: Some(0),
            active_parameter,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_include;
    use lsp_server::{Connection, Message};
    use lsp_types::{Position, SignatureHelpParams, TextDocumentPositionParams};
    use lsp_types::{
        TextDocumentIdentifier, Uri, WorkDoneProgressParams,
        request::{Request, SignatureHelpRequest},
    };
    use nu_test_support::fs::fixtures;

    fn send_signature_help_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: SignatureHelpRequest::METHOD.to_string(),
                params: serde_json::to_value(SignatureHelpParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
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
    fn signature_help_on_builtins() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("signature.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_signature_help_request(&client_connection, script.clone(), 0, 15);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "signatures": [{
                    "label": "str substring {flags} <range> ...(rest)",
                    "parameters": [ ],
                    "activeParameter": 0
                }],
                "activeSignature": 0
            })
        );

        let resp = send_signature_help_request(&client_connection, script.clone(), 0, 17);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{ "activeParameter": 0 }], "activeSignature": 0 })
        );

        let resp = send_signature_help_request(&client_connection, script.clone(), 0, 18);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{ "activeParameter": 1 }], "activeSignature": 0 })
        );

        let resp = send_signature_help_request(&client_connection, script.clone(), 0, 22);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{ "activeParameter": 1 }], "activeSignature": 0 })
        );

        let resp = send_signature_help_request(&client_connection, script.clone(), 7, 0);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{
                "label": "str substring {flags} <range> ...(rest)",
                "activeParameter": 1
            }]})
        );

        let resp = send_signature_help_request(&client_connection, script.clone(), 4, 0);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{
                "label": "str substring {flags} <range> ...(rest)",
                "activeParameter": 0
            }]})
        );

        let resp = send_signature_help_request(&client_connection, script, 16, 6);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({ "signatures": [{
                "label": "echo {flags} ...(rest)",
                "activeParameter": 0
            }]})
        );
    }

    #[test]
    fn signature_help_on_custom_commands() {
        let config_str = r#"export def "foo bar" [
    p1: int
    p2: string, # doc
    p3?: int = 1
] {}"#;
        let (client_connection, _recv) = initialize_language_server(Some(config_str), None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hints");
        script.push("signature.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_signature_help_request(&client_connection, script.clone(), 9, 11);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "signatures": [{
                    "label": "foo bar {flags} <p1> <p2> (p3)",
                    "parameters": [
                        {"label": "p1", "documentation": {"value": ": `<int>`"}},
                        {"label": "p2", "documentation": {"value": ": `<string>` - doc"}},
                        {"label": "p3", "documentation": {"value": ": `<int>` - (optional, default: `1`)"}},
                    ],
                    "activeParameter": 1
                }],
                "activeSignature": 0,
                "activeParameter": 1
            })
        );

        let resp = send_signature_help_request(&client_connection, script, 10, 15);
        assert_json_include!(
            actual: result_from_message(resp),
            expected: serde_json::json!({
                "signatures": [{
                    "label": "foo baz {flags} <p1> <p2> (p3)",
                    "parameters": [
                        {"label": "p1", "documentation": {"value": ": `<int>`"}},
                        {"label": "p2", "documentation": {"value": ": `<string>` - doc"}},
                        {"label": "p3", "documentation": {"value": ": `<int>` - (optional, default: `1`)"}},
                        {"label": "-h, --help", "documentation": {"value": " - Display the help message for this command"}},
                    ],
                    "activeParameter": 2
                }],
                "activeSignature": 0,
                "activeParameter": 2
            })
        );
    }
}
