use crate::LanguageServer;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidChangeWorkspaceFolders, DidCloseTextDocument,
        DidOpenTextDocument, Notification, Progress,
    },
    DidChangeTextDocumentParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, ProgressParams, ProgressParamsValue, ProgressToken, Uri,
    WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd, WorkDoneProgressReport,
};
use miette::{IntoDiagnostic, Result};

impl LanguageServer {
    pub(crate) fn handle_lsp_notification(
        &mut self,
        notification: lsp_server::Notification,
    ) -> Option<Uri> {
        let mut docs = self.docs.lock().ok()?;
        docs.listen(notification.method.as_str(), &notification.params);
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidOpenTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidChangeTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidCloseTextDocumentParams");
                let uri = params.text_document.uri;
                self.symbol_cache.drop(&uri);
                self.inlay_hints.remove(&uri);
                None
            }
            DidChangeWorkspaceFolders::METHOD => {
                let params: DidChangeWorkspaceFoldersParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidChangeWorkspaceFoldersParams");
                for added in params.event.added {
                    self.workspace_folders.insert(added.name.clone(), added);
                }
                for removed in params.event.removed {
                    self.workspace_folders.remove(&removed.name);
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn send_progress_notification(
        &self,
        token: ProgressToken,
        value: WorkDoneProgress,
    ) -> Result<()> {
        let progress_params = ProgressParams {
            token,
            value: ProgressParamsValue::WorkDone(value),
        };
        let notification =
            lsp_server::Notification::new(Progress::METHOD.to_string(), progress_params);
        self.connection
            .sender
            .send(lsp_server::Message::Notification(notification))
            .into_diagnostic()
    }

    pub(crate) fn send_progress_begin(&self, token: ProgressToken, title: String) -> Result<()> {
        self.send_progress_notification(
            token,
            WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title,
                percentage: Some(0),
                cancellable: Some(true),
                ..Default::default()
            }),
        )
    }

    pub(crate) fn send_progress_report(
        &self,
        token: ProgressToken,
        percentage: u32,
        message: Option<String>,
    ) -> Result<()> {
        self.send_progress_notification(
            token,
            WorkDoneProgress::Report(WorkDoneProgressReport {
                message,
                cancellable: Some(true),
                percentage: Some(percentage),
            }),
        )
    }

    pub(crate) fn send_progress_end(
        &self,
        token: ProgressToken,
        message: Option<String>,
    ) -> Result<()> {
        self.send_progress_notification(
            token,
            WorkDoneProgress::End(WorkDoneProgressEnd { message }),
        )
    }

    pub(crate) fn send_error_message(
        &self,
        id: lsp_server::RequestId,
        code: i32,
        message: String,
    ) -> Result<()> {
        self.connection
            .sender
            .send(lsp_server::Message::Response(lsp_server::Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code,
                    message,
                    data: None,
                }),
            }))
            .into_diagnostic()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{
        initialize_language_server, open, open_unchecked, result_from_message, send_hover_request,
        update,
    };
    use assert_json_diff::assert_json_eq;
    use lsp_types::Range;
    use nu_test_support::fs::fixtures;

    #[test]
    fn hover_correct_documentation_on_let() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_hover_request(&client_connection, script.clone(), 0, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Create a variable and give it a value.\n\nThis command is a parser keyword. For details, check:\n  https://www.nushell.sh/book/thinking_in_nu.html\n---\n### Usage \n```nu\n  let {flags} <var_name> <initial_value>\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `var_name: any` - Variable name.\n\n  `initial_value: any` - Equals sign followed by value.\n\n\n### Input/output types\n\n```nu\n any | nothing\n\n```\n### Example(s)\n  Set a variable to a value\n```nu\n  let x = 10\n```\n  Set a variable to the result of an expression\n```nu\n  let x = 10 + 100\n```\n  Set a variable based on the condition\n```nu\n  let x = if false { -1 } else { 1 }\n```\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_command_after_full_content_change() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        update(
            &client_connection,
            script.clone(),
            String::from(
                r#"# Renders some updated greeting message
def hello [] {}

hello"#,
            ),
            None,
        );
        let resp = send_hover_request(&client_connection, script.clone(), 3, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Renders some updated greeting message\n---\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_command_after_partial_content_change() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        update(
            &client_connection,
            script.clone(),
            String::from("# Renders some updated greeting message"),
            Some(Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 31,
                },
            }),
        );
        let resp = send_hover_request(&client_connection, script.clone(), 3, 0);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Renders some updated greeting message\n---\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn open_document_with_utf_char() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("notifications");
        script.push("issue_11522.nu");
        let script = path_to_uri(&script);

        let result = open(&client_connection, script);

        assert_eq!(result.map(|_| ()), Ok(()))
    }
}
