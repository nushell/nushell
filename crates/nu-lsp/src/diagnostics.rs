use crate::{LanguageServer, span_to_range};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams, Uri,
    notification::{Notification, PublishDiagnostics},
};
use miette::{IntoDiagnostic, Result, miette};

impl LanguageServer {
    pub(crate) fn publish_diagnostics_for_file(&mut self, uri: Uri) -> Result<()> {
        let mut engine_state = self.new_engine_state(Some(&uri));
        engine_state.generate_nu_constant();

        let Some((_, span, working_set)) = self.parse_file(&mut engine_state, &uri, true) else {
            return Ok(());
        };

        let mut diagnostics = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: Vec::new(),
            version: None,
        };

        let docs = match self.docs.lock() {
            Ok(it) => it,
            Err(err) => return Err(miette!(err.to_string())),
        };
        let file = docs
            .get_document(&uri)
            .ok_or_else(|| miette!("\nFailed to get document"))?;
        for err in working_set.parse_errors.iter() {
            let message = err.to_string();

            diagnostics.diagnostics.push(Diagnostic {
                range: span_to_range(&err.span(), file, span.start),
                severity: Some(DiagnosticSeverity::ERROR),
                message,
                ..Default::default()
            });
        }

        for warn in working_set.parse_warnings.iter() {
            let message = warn.to_string();

            diagnostics.diagnostics.push(Diagnostic {
                range: span_to_range(&warn.span(), file, span.start),
                severity: Some(DiagnosticSeverity::WARNING),
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
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, update};
    use assert_json_diff::assert_json_eq;
    use nu_test_support::fs::fixtures;
    use rstest::rstest;

    #[rstest]
    #[case::file_with_no_issues("pwd.nu", None, serde_json::json!([]))]
    #[case::file_fixed_by_update("var.nu", Some(("$env", lsp_types::Range {
        start: lsp_types::Position { line: 0, character: 6 },
        end: lsp_types::Position { line: 0, character: 30 },
    })), serde_json::json!([]))]
    #[case::variable_does_not_exist("var.nu", None, serde_json::json!([{
        "range": {
            "start": { "line": 0, "character": 6 },
            "end": { "line": 0, "character": 30 }
        },
        "message": "Variable not found.",
        "severity": 1
    }]))]
    fn publish_diagnostics(
        #[case] filename: &str,
        #[case] update_op: Option<(&str, lsp_types::Range)>,
        #[case] expected_diagnostics: serde_json::Value,
    ) {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp/diagnostics");
        script.push(filename);
        let script = path_to_uri(&script);

        let mut notification = open_unchecked(&client_connection, script.clone());
        // For files that need fixing, open first then update
        if let Some((text, range)) = update_op {
            notification = update(
                &client_connection,
                script.clone(),
                String::from(text),
                Some(range),
            );
        };

        assert_json_eq!(
            notification,
            serde_json::json!({
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": script,
                    "diagnostics": expected_diagnostics
                }
            })
        );
    }
}
