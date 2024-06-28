use crate::LanguageServer;
use nu_cli::eval_config_contents;
use nu_protocol::engine::EngineState;
use serde::Deserialize;
use std::path::PathBuf;

pub(crate) const CONFIG_REQUEST_ID: &'static str = "config";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NuConfiguration {
    include_paths: Vec<PathBuf>,
}

impl LanguageServer {
    pub(crate) fn configure_engine(engine_state: &mut EngineState, config: serde_json::Value) {
        let Ok(config) = serde_json::from_value::<NuConfiguration>(config) else {
            // TODO: warn the client? Whet does the spec recommend?
            return;
        };

        for config_path in config.include_paths {
            let mut stack = nu_protocol::engine::Stack::new();
            eval_config_contents(config_path, engine_state, &mut stack);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::{complete, initialize_language_server, open_unchecked};
    use assert_json_diff::assert_json_include;
    use lsp_server::Message;
    use lsp_types::Url;
    use nu_test_support::fs::fixtures;

    #[test]
    fn complete_with_include_paths() {
        let mut include_path = fixtures();
        include_path.push("formats");
        include_path.push("sample_def.nu");

        let (client_connection, _recv) = initialize_language_server(Some(serde_json::json!({
            "includePaths": [
                include_path
            ]
        })));

        let mut script = fixtures();
        script.push("lsp");
        script.push("completion");
        script.push("include.nu");
        let script = Url::from_file_path(script).unwrap();

        open_unchecked(&client_connection, script.clone());

        let Message::Response(resp) = complete(&client_connection, script, 0, 3) else {
            panic!()
        };

        assert_json_include!(
            actual: resp.result,
            expected: serde_json::json!([
               {
                  "label": "greet",
                  "textEdit": {
                     "newText": "greet",
                     "range": {
                        "start": { "character": 0, "line": 0 },
                        "end": { "character": 3, "line": 0 }
                     }
                  }
               }
            ])
        );
    }
}
