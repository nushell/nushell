use crate::LanguageServer;
use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams, Url,
};
use miette::{IntoDiagnostic, Result};
use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span, Value,
};

impl LanguageServer {
    pub(crate) fn publish_diagnostics_for_file(
        &self,
        uri: Url,
        engine_state: &mut EngineState,
    ) -> Result<()> {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
        engine_state.generate_nu_constant();

        let mut working_set = StateWorkingSet::new(engine_state);

        let Some((rope_of_file, file_path)) = self.rope(&uri) else {
            return Ok(());
        };

        let contents = rope_of_file.bytes().collect::<Vec<u8>>();
        let offset = working_set.next_span_start();
        working_set.files.push(file_path.into(), Span::unknown())?;
        parse(
            &mut working_set,
            Some(&file_path.to_string_lossy()),
            &contents,
            false,
        );

        let mut diagnostics = PublishDiagnosticsParams {
            uri,
            diagnostics: Vec::new(),
            version: None,
        };

        for err in working_set.parse_errors.iter() {
            let message = err.to_string();

            diagnostics.diagnostics.push(Diagnostic {
                range: Self::span_to_range(
                    &err.span(),
                    rope_of_file,
                    offset,
                    &self.position_encoding,
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                message,
                ..Default::default()
            });
        }

        self.connection
            .sender
            .send(lsp_server::Message::Notification(
                lsp_server::Notification::new(PublishDiagnostics::METHOD.to_string(), diagnostics),
            ))
            .into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use lsp_types::Url;
    use nu_test_support::fs::fixtures;

    use crate::tests::{initialize_language_server, open_unchecked, update};

    #[test]
    fn publish_diagnostics_variable_does_not_exists() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("diagnostics");
        script.push("var.nu");
        let script = Url::from_file_path(script).unwrap();

        let notification = open_unchecked(&client_connection, script.clone());

        assert_json_eq!(
            notification,
            serde_json::json!({
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": script,
                    "diagnostics": [{
                        "range": {
                            "start": { "line": 0, "character": 6 },
                            "end": { "line": 0, "character": 30 }
                        },
                        "message": "Variable not found.",
                        "severity": 1
                    }]
                }
            })
        );
    }

    #[test]
    fn publish_diagnostics_fixed_unknown_variable() {
        let (client_connection, _recv) = initialize_language_server(None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("diagnostics");
        script.push("var.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());
        let notification = update(
            &client_connection,
            script.clone(),
            String::from("$env"),
            Some(lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 6,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 30,
                },
            }),
        );

        assert_json_eq!(
            notification,
            serde_json::json!({
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": script,
                    "diagnostics": []
                }
            })
        );
    }
}
