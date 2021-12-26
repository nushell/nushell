use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};

const NEWLINE_ESCAPE_CODE: &str = "<\\n>";

fn decode_newlines(escaped: &str) -> String {
    escaped.replace(NEWLINE_ESCAPE_CODE, "\n")
}

#[derive(Clone)]
pub struct History;

impl Command for History {
    fn name(&self) -> &str {
        "history"
    }

    fn usage(&self) -> &str {
        "Get the command history"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history")
            .switch("clear", "Clears out the history entries", Some('c'))
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        if let Some(config_path) = nu_path::config_dir() {
            let clear = call.has_flag("clear");
            let ctrlc = engine_state.ctrlc.clone();

            let mut history_path = config_path;
            history_path.push("nushell");
            history_path.push("history.txt");

            if clear {
                let _ = std::fs::remove_file(history_path);
                Ok(PipelineData::new(head))
            } else {
                let contents = std::fs::read_to_string(history_path);

                if let Ok(contents) = contents {
                    Ok(contents
                        .lines()
                        .map(move |x| Value::String {
                            val: decode_newlines(x),
                            span: head,
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .into_pipeline_data(ctrlc))
                } else {
                    Err(ShellError::FileNotFound(head))
                }
            }
        } else {
            Err(ShellError::FileNotFound(head))
        }
    }
}
