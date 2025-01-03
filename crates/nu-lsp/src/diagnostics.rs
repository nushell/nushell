use crate::LanguageServer;
use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams, Uri,
};
use miette::{IntoDiagnostic, Result};
use nu_protocol::Value;

impl LanguageServer {
    pub(crate) fn publish_diagnostics_for_file(&mut self, uri: Uri) -> Result<()> {
        let mut engine_state = self.engine_state.clone();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
        engine_state.generate_nu_constant();

        let Some((_, offset, working_set, file)) =
            self.update_engine_state(&mut engine_state, uri.clone())
        else {
            return Ok(());
        };

        let mut diagnostics = PublishDiagnosticsParams {
            uri,
            diagnostics: Vec::new(),
            version: None,
        };

        for err in working_set.parse_errors.iter() {
            let message = err.to_string();

            diagnostics.diagnostics.push(Diagnostic {
                range: Self::span_to_range(&err.span(), file, offset),
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
    use nu_test_support::fs::fixtures;

    use crate::tests::{initialize_language_server, open_unchecked, pathbuf_to_uri, update};

    #[test]
    fn publish_diagnostics_variable_does_not_exists() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("diagnostics");
        script.push("var.nu");
        let script = pathbuf_to_uri(&script);

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
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("diagnostics");
        script.push("var.nu");
        let script = pathbuf_to_uri(&script);

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
