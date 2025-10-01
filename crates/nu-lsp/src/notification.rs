use crate::LanguageServer;
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, LogMessageParams, MessageType, ProgressParams, ProgressParamsValue,
    ProgressToken, Uri, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkDoneProgressReport,
    notification::{
        DidChangeTextDocument, DidChangeWorkspaceFolders, DidCloseTextDocument,
        DidOpenTextDocument, Notification, Progress,
    },
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
                let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)
                    .expect("Expect receive DidOpenTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params)
                        .expect("Expect receive DidChangeTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams =
                    serde_json::from_value(notification.params)
                        .expect("Expect receive DidCloseTextDocumentParams");
                let uri = params.text_document.uri;
                self.symbol_cache.drop(&uri);
                self.inlay_hints.remove(&uri);
                None
            }
            DidChangeWorkspaceFolders::METHOD => {
                let params: DidChangeWorkspaceFoldersParams =
                    serde_json::from_value(notification.params)
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

    pub(crate) fn send_log_message(&self, typ: MessageType, message: String) -> Result<()> {
        let log_params = LogMessageParams { typ, message };
        let notification = lsp_server::Notification::new(
            lsp_types::notification::LogMessage::METHOD.to_string(),
            log_params,
        );
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
        initialize_language_server, open_unchecked, result_from_message, send_hover_request, update,
    };
    use assert_json_diff::assert_json_eq;
    use lsp_types::Range;
    use nu_test_support::fs::fixtures;
    use rstest::rstest;

    #[rstest]
    #[case::full(
        r#"# Renders some updated greeting message
def hello [] {}

hello"#,
        None
    )]
    #[case::partial(
        "# Renders some updated greeting message",
        Some(Range {
            start: lsp_types::Position {
                line: 0,
                character: 0,
            },
            end: lsp_types::Position {
                line: 0,
                character: 31,
            },
        })
    )]
    fn hover_on_command_after_content_change(#[case] text: String, #[case] range: Option<Range>) {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp/hover/command.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        update(&client_connection, script.clone(), text, range);
        let resp = send_hover_request(&client_connection, script, 3, 0);

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
        script.push("lsp/notifications/issue_11522.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script);
    }
}
