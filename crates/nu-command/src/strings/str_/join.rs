use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::RawStream;

#[derive(Clone)]
pub struct StrJoin;

impl Command for StrJoin {
    fn name(&self) -> &str {
        "str join"
    }

    fn signature(&self) -> Signature {
        Signature::build("str join")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::String),
                (Type::String, Type::String),
            ])
            .switch(
                "stream",
                "Output the result as a raw stream, instead of collecting to a string.",
                Some('s'),
            )
            .optional(
                "separator",
                SyntaxShape::String,
                "Optional separator to use when creating string.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Concatenate multiple strings into a single string, with an optional separator between each."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["collect", "concatenate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let should_stream = call.has_flag(engine_state, stack, "stream")?;
        let separator: Option<String> = call.opt(engine_state, stack, 0)?;

        let config = engine_state.config.clone();
        let metadata = input.metadata();

        // Create an iterator that contains individual chunks, interspersing the separator if it
        // was specified.
        //
        // This iterator doesn't borrow anything, so we can also use it to construct the
        // `RawStream`.
        let iter = Itertools::intersperse(
            input.into_iter().map(move |value| {
                // This is wrapped in Some so that we can intersperse an optional separator and then
                // flatten it without that
                Some(match value {
                    // Propagate errors
                    Value::Error { error, .. } => Err(*error),
                    // Format dates using their debug format
                    Value::Date { val, .. } => Ok(format!("{val:?}")),
                    // Use `to_expanded_string()` on all other values
                    value => Ok(value.to_expanded_string("\n", &config)),
                })
            }),
            separator.map(Ok),
        )
        .flatten();

        if should_stream {
            Ok(PipelineData::ExternalStream {
                stdout: Some(RawStream::new(
                    Box::new(iter.map(|result| result.map(|string| string.into_bytes()))),
                    engine_state.ctrlc.clone(),
                    call.head,
                    None,
                )),
                stderr: None,
                exit_code: None,
                span: call.head,
                metadata,
                trim_end_newline: false,
            })
        } else {
            let string = iter.collect::<Result<String, ShellError>>()?;
            Ok(Value::string(string, call.head).into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a string from input",
                example: "['nu', 'shell'] | str join",
                result: Some(Value::test_string("nushell")),
            },
            Example {
                description: "Create a string from input with a separator",
                example: "['nu', 'shell'] | str join '-'",
                result: Some(Value::test_string("nu-shell")),
            },
            Example {
                description: "Join a stream of numbers with commas",
                example: r#"seq 1 5 | str join --stream ", ""#,
                result: Some(Value::test_string("1, 2, 3, 4, 5")),
            },
            Example {
                description: "Join an infinite stream of numbers with commas",
                example: r#"1.. | each {} | str join --stream ", ""#,
                result: None,
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

        test_examples(StrJoin {})
    }
}
