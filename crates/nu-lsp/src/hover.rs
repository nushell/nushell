use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use nu_protocol::{PositionalArg, engine::Command};

use crate::{
    Id, LanguageServer,
    signature::{display_flag, doc_for_arg, get_signature_label},
};

impl LanguageServer {
    pub(crate) fn get_decl_description(decl: &dyn Command, skip_description: bool) -> String {
        let mut description = String::new();

        if !skip_description {
            // First description
            description.push_str(&format!("{}\n", decl.description().replace('\r', "")));

            // Additional description
            if !decl.extra_description().is_empty() {
                description.push_str(&format!("\n{}\n", decl.extra_description()));
            }
        }
        // Usage
        description.push_str("---\n### Usage \n```nu\n");
        let signature = decl.signature();
        description.push_str(&get_signature_label(&signature, true));
        description.push_str("\n```\n");

        // Flags
        if !signature.named.is_empty() {
            description.push_str("\n### Flags\n\n");
            let mut first = true;
            for named in signature.named {
                if first {
                    first = false;
                } else {
                    description.push('\n');
                }
                description.push_str("  ");
                description.push_str(&display_flag(&named, true));
                description.push_str(&doc_for_arg(
                    named.arg,
                    named.desc,
                    named.default_value,
                    false,
                ));
                description.push('\n');
            }
            description.push('\n');
        }

        // Parameters
        if !signature.required_positional.is_empty()
            || !signature.optional_positional.is_empty()
            || signature.rest_positional.is_some()
        {
            description.push_str("\n### Parameters\n\n");
            let mut first = true;
            let mut write_arg = |arg: PositionalArg, optional: bool| {
                if first {
                    first = false;
                } else {
                    description.push('\n');
                }
                description.push_str(&format!("  `{}`", arg.name));
                description.push_str(&doc_for_arg(
                    Some(arg.shape),
                    arg.desc,
                    arg.default_value,
                    optional,
                ));
                description.push('\n');
            };
            for required_arg in signature.required_positional {
                write_arg(required_arg, false);
            }
            for optional_arg in signature.optional_positional {
                write_arg(optional_arg, true);
            }
            if let Some(arg) = signature.rest_positional {
                if !first {
                    description.push('\n');
                }
                description.push_str(&format!(" `...{}`", arg.name));
                description.push_str(&doc_for_arg(
                    Some(arg.shape),
                    arg.desc,
                    arg.default_value,
                    false,
                ));
                description.push('\n');
            }
            description.push('\n');
        }

        // Input/output types
        if !signature.input_output_types.is_empty() {
            description.push_str("\n### Input/output types\n");
            description.push_str("\n```nu\n");
            for input_output in &signature.input_output_types {
                description.push_str(&format!(" {} | {}\n", input_output.0, input_output.1));
            }
            description.push_str("\n```\n");
        }

        // Examples
        if !decl.examples().is_empty() {
            description.push_str("### Example(s)\n");
            for example in decl.examples() {
                description.push_str(&format!(
                    "  {}\n```nu\n  {}\n```\n",
                    example.description, example.example
                ));
            }
        }
        description
    }

    pub(crate) fn hover(&mut self, params: &HoverParams) -> Option<Hover> {
        let path_uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_owned();
        let mut engine_state = self.new_engine_state(Some(&path_uri));
        let (working_set, id, _, _) = self
            .parse_and_find(
                &mut engine_state,
                &path_uri,
                params.text_document_position_params.position,
            )
            .ok()?;

        let markdown_hover = |content: String| {
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                // TODO
                range: None,
            })
        };

        match id {
            Id::Variable(var_id, _) => {
                let var = working_set.get_variable(var_id);
                let value = var
                    .const_val
                    .clone()
                    .and_then(|v| v.coerce_into_string().ok())
                    .unwrap_or(String::from(if var.mutable {
                        "mutable"
                    } else {
                        "immutable"
                    }));
                let contents = format!("```\n{}\n``` \n---\n{}", var.ty, value);
                markdown_hover(contents)
            }
            Id::CellPath(var_id, cell_path) => {
                let var = working_set.get_variable(var_id);
                markdown_hover(
                    var.const_val
                        .as_ref()
                        .and_then(|val| val.follow_cell_path(&cell_path).ok())
                        .map(|val| {
                            let ty = val.get_type();
                            if let Ok(s) = val.coerce_str() {
                                format!("```\n{ty}\n```\n---\n{s}")
                            } else {
                                format!("```\n{ty}\n```")
                            }
                        })
                        .unwrap_or("`unknown`".into()),
                )
            }
            Id::Declaration(decl_id) => markdown_hover(Self::get_decl_description(
                working_set.get_decl(decl_id),
                false,
            )),
            Id::Module(module_id, _) => {
                let description = working_set
                    .get_module_comments(module_id)?
                    .iter()
                    .map(|sp| String::from_utf8_lossy(working_set.get_span_contents(*sp)).into())
                    .collect::<Vec<String>>()
                    .join("\n");
                markdown_hover(description)
            }
            Id::Value(t) => markdown_hover(format!("`{t}`")),
            Id::External(cmd) => {
                let command_output = if cfg!(windows) {
                    std::process::Command::new("powershell.exe")
                        .args(["-NoProfile", "-Command", "help", &cmd])
                        .output()
                } else {
                    std::process::Command::new("man").arg(&cmd).output()
                };
                let manpage_str = match command_output {
                    Ok(output) => nu_utils::strip_ansi_likely(
                        String::from_utf8_lossy(&output.stdout).as_ref(),
                    )
                    .to_string(),
                    Err(_) => format!("No command help found for {}", &cmd),
                };
                markdown_hover(manpage_str)
            }
        }
    }
}

#[cfg(test)]
mod hover_tests {
    use crate::{
        path_to_uri,
        tests::{
            initialize_language_server, open_unchecked, result_from_message, send_hover_request,
        },
    };
    use assert_json_diff::assert_json_eq;
    use nu_test_support::fs::fixtures;

    #[test]
    fn hover_on_variable() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script, 2, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({ "contents": { "kind": "markdown", "value": "```\ntable\n``` \n---\nimmutable" } })
        );
    }

    #[test]
    fn hover_on_cell_path() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("use.nu");
        let script = path_to_uri(&script);
        open_unchecked(&client_connection, script.clone());

        let resp = send_hover_request(&client_connection, script.clone(), 2, 3);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nlist<any>\n```")
        );

        let resp = send_hover_request(&client_connection, script.clone(), 2, 7);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nrecord<bar: int>\n```")
        );

        let resp = send_hover_request(&client_connection, script, 2, 11);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nint\n```\n---\n2")
        );

        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        script.push("baz.nu");
        let script = path_to_uri(&script);
        open_unchecked(&client_connection, script.clone());

        // For module record
        let resp = send_hover_request(&client_connection, script, 8, 42);
        let result = result_from_message(resp);
        assert_json_eq!(
            result.pointer("/contents/value").unwrap(),
            serde_json::json!("```\nstring\n```\n---\nconst value")
        );
    }

    #[test]
    fn hover_on_custom_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script, 3, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Renders some greeting message\n---\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_custom_in_custom() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script, 9, 7);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "\n---\n### Usage \n```nu\n  bar {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_external_command() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script, 6, 2);

        let hover_text = result_from_message(resp)
            .pointer("/contents/value")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        #[cfg(not(windows))]
        assert!(hover_text.contains("SLEEP"));
        #[cfg(windows)]
        assert!(hover_text.contains("Start-Sleep"));
    }

    #[test]
    fn hover_on_str_join() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script, 5, 8);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                    "contents": {
                    "kind": "markdown",
                    "value": "Concatenate multiple strings into a single string, with an optional separator between each.\n---\n### Usage \n```nu\n  str join {flags} (separator)\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `separator`: `<string>` - Optional separator to use when creating string. (optional)\n\n\n### Input/output types\n\n```nu\n list<any> | string\n string | string\n\n```\n### Example(s)\n  Create a string from input\n```nu\n  ['nu', 'shell'] | str join\n```\n  Create a string from input with a separator\n```nu\n  ['nu', 'shell'] | str join '-'\n```\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_module() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("workspace");
        script.push("foo.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 15, 15);
        let result = result_from_message(resp);
        assert_eq!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"# cmt\""
        );

        let resp = send_hover_request(&client_connection, script.clone(), 17, 27);
        let result = result_from_message(resp);
        assert_eq!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"# sub cmt\""
        );

        let resp = send_hover_request(&client_connection, script, 19, 33);
        let result = result_from_message(resp);
        assert_eq!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"# sub sub cmt\""
        );
    }

    #[test]
    fn hover_on_use_command() {
        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("use.nu");
        let script_uri = path_to_uri(&script);

        let config = format!("use {}", script.to_str().unwrap());
        let (client_connection, _recv) = initialize_language_server(Some(&config), None);

        open_unchecked(&client_connection, script_uri.clone());
        let resp = send_hover_request(&client_connection, script_uri.clone(), 0, 19);
        let result = result_from_message(resp);

        assert_eq!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", ""),
            "\"```\\nrecord<foo: list<any>>\\n``` \\n---\\nimmutable\""
        );

        let resp = send_hover_request(&client_connection, script_uri, 0, 22);
        let result = result_from_message(resp);

        assert!(
            result
                .pointer("/contents/value")
                .unwrap()
                .to_string()
                .replace("\\r", "")
                .starts_with("\"\\n---\\n### Usage \\n```nu\\n  foo {flags}\\n```\\n\\n### Flags")
        );
    }
}
