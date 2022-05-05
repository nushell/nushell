use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct ToText;

impl Command for ToText {
    fn name(&self) -> &str {
        "to text"
    }

    fn signature(&self) -> Signature {
        Signature::build("to text").category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts data into simple text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let span = call.head;
        let config = engine_state.get_config();

        let line_ending = if cfg!(target_os = "windows") {
            "\r\n"
        } else {
            "\n"
        };

        let collected_input = input.collect_string(line_ending, config)?;
        Ok(Value::String {
            val: collected_input,
            span,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs data as simple text",
                example: "1 | to text",
                result: Some(Value::test_string("1")),
            },
            Example {
                description: "Outputs external data as simple text",
                example: "git help -a | lines | find -r '^ ' |  to text",
                result: None,
            },
            Example {
                description: "Outputs records as simple text",
                example: "ls |  to text",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "convert"]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToText {})
    }
}
