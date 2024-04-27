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

        // Create an iterator that contains individual chunks, prepending the separator for chunks
        // after the first one.
        //
        // This iterator doesn't borrow anything, so we can also use it to construct the
        // `RawStream`.
        let mut first = true;
        let iter = input.into_iter().map(move |value| {
            use std::fmt::Write;

            let mut string = if first {
                first = false;
                String::new()
            } else if let Some(separator) = separator.as_ref() {
                separator.clone()
            } else {
                String::new()
            };

            match value {
                Value::Error { error, .. } => {
                    return Err(*error);
                }
                Value::Date { val, .. } => {
                    // very unlikely that this fails, and format!() panics anyway
                    write!(string, "{val:?}").expect("formatting failed");
                }
                value => string.push_str(&value.to_expanded_string("\n", &config)),
            }
            Ok(string)
        });

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
