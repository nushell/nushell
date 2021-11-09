use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct StrCollect;

impl Command for StrCollect {
    fn name(&self) -> &str {
        "str collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("str collect").optional(
            "separator",
            SyntaxShape::String,
            "optional separator to use when creating string",
        )
    }

    fn usage(&self) -> &str {
        "creates a string from the input, optionally using a separator"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<String> = call.opt(engine_state, stack, 0)?;

        #[allow(clippy::needless_collect)]
        let strings: Vec<Result<String, ShellError>> =
            input.into_iter().map(|value| value.as_string()).collect();
        let strings: Result<Vec<_>, _> = strings.into_iter().collect::<Result<_, _>>();

        match strings {
            Ok(strings) => {
                let output = if let Some(separator) = separator {
                    strings.join(&separator)
                } else {
                    strings.join("")
                };

                Ok(Value::String {
                    val: output,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            _ => Err(ShellError::CantConvert(
                "string".into(),
                "non-string input".into(),
                call.head,
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a string from input",
                example: "['nu', 'shell'] | str collect",
                result: Some(Value::String {
                    val: "nushell".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Create a string from input with a separator",
                example: "['nu', 'shell'] | str collect '-'",
                result: Some(Value::String {
                    val: "nu-shell".to_string(),
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrCollect {})
    }
}
