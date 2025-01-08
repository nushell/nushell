use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification},
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, Uri,
};

use crate::LanguageServer;

impl LanguageServer {
    pub(crate) fn handle_lsp_notification(
        &mut self,
        notification: lsp_server::Notification,
    ) -> Option<Uri> {
        self.docs
            .listen(notification.method.as_str(), &notification.params);
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidOpenTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidSaveTextDocumentParams");
                Some(params.text_document.uri)
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params.clone())
                        .expect("Expect receive DidChangeTextDocumentParams");
                Some(params.text_document.uri)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use lsp_server::Message;
    use lsp_types::Range;
    use nu_test_support::fs::fixtures;

    use crate::path_to_uri;
    use crate::tests::{hover, initialize_language_server, open, open_unchecked, update};

    #[test]
    fn hover_correct_documentation_on_let() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("var.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = hover(&client_connection, script.clone(), 0, 0);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Create a variable and give it a value.\n\nThis command is a parser keyword. For details, check:\n  https://www.nushell.sh/book/thinking_in_nu.html\n### Usage \n```nu\n  let {flags} <var_name> <initial_value>\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n\n### Parameters\n\n  `var_name: any` - Variable name.\n\n  `initial_value: any` - Equals sign followed by value.\n\n\n### Input/output types\n\n```nu\n any | nothing\n\n```\n### Example(s)\n  Set a variable to a value\n```nu\n  let x = 10\n```\n  Set a variable to the result of an expression\n```nu\n  let x = 10 + 100\n```\n  Set a variable based on the condition\n```nu\n  let x = if false { -1 } else { 1 }\n```\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_command_after_full_content_change() {
        let (client_connection, _recv) = initialize_language_server();

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

        let resp = hover(&client_connection, script.clone(), 3, 0);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Renders some updated greeting message\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn hover_on_command_after_partial_content_change() {
        let (client_connection, _recv) = initialize_language_server();

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

        let resp = hover(&client_connection, script.clone(), 3, 0);
        let result = if let Message::Response(response) = resp {
            response.result
        } else {
            panic!()
        };

        assert_json_eq!(
            result,
            serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "Renders some updated greeting message\n### Usage \n```nu\n  hello {flags}\n```\n\n### Flags\n\n  `-h`, `--help` - Display the help message for this command\n\n"
                }
            })
        );
    }

    #[test]
    fn open_document_with_utf_char() {
        let (client_connection, _recv) = initialize_language_server();

        let mut script = fixtures();
        script.push("lsp");
        script.push("notifications");
        script.push("issue_11522.nu");
        let script = path_to_uri(&script);

        let result = open(&client_connection, script);

        assert_eq!(result.map(|_| ()), Ok(()))
    }
}
