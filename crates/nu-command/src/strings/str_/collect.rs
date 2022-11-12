use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct StrCollect;

impl Command for StrCollect {
    fn name(&self) -> &str {
        "str collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("str collect")
            .input_output_types(vec![(Type::List(Box::new(Type::String)), Type::String)])
            .optional(
                "separator",
                SyntaxShape::String,
                "optional separator to use when creating string",
            )
            .category(Category::Deprecated)
    }

    fn usage(&self) -> &str {
        "'str collect' is deprecated. Please use 'str join' instead."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<String> = call.opt(engine_state, stack, 0)?;

        let config = engine_state.get_config();

        // let output = input.collect_string(&separator.unwrap_or_default(), &config)?;
        // Hmm, not sure what we actually want. If you don't use debug_string, Date comes out as human readable
        // which feels funny
        let mut strings: Vec<String> = vec![];

        for value in input {
            match value {
                Value::Error { error } => {
                    return Err(error);
                }
                value => {
                    strings.push(value.debug_string("\n", config));
                }
            }
        }

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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a string from input",
                example: "['nu', 'shell'] | str collect",
                result: Some(Value::String {
                    val: "nushell".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Create a string from input with a separator",
                example: "['nu', 'shell'] | str collect '-'",
                result: Some(Value::String {
                    val: "nu-shell".to_string(),
                    span: Span::test_data(),
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
