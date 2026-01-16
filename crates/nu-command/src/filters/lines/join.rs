use nu_engine::command_prelude::*;
use nu_protocol::{ByteStreamType, Signals, shell_error::io::IoError};
use std::io::Write;

#[derive(Clone)]
pub struct LinesJoin;

impl Command for LinesJoin {
    fn name(&self) -> &str {
        "lines join"
    }

    fn signature(&self) -> Signature {
        Signature::build("lines join")
            .input_output_types(vec![
                (Type::List(Box::new(Type::String)), Type::String),
                (Type::List(Box::new(Type::Any)), Type::String),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Join a list of strings into a single string with newlines."
    }

    fn extra_description(&self) -> &str {
        "This is the inverse of the `lines` command. It takes a list of strings \
        and joins them with newline characters."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["join", "concat", "merge", "combine", "newline"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = engine_state.config.clone();
        let span = call.head;
        let metadata = input.metadata();
        let mut iter = input.into_iter();
        let mut first = true;

        let output = ByteStream::from_fn(
            span,
            Signals::empty(),
            ByteStreamType::String,
            move |buffer| {
                let from_io_error = IoError::factory(span, None);

                if let Some(value) = iter.next() {
                    if first {
                        first = false;
                    } else {
                        writeln!(buffer).map_err(&from_io_error)?;
                    }

                    match value {
                        Value::Error { error, .. } => {
                            return Err(*error);
                        }
                        value => write!(buffer, "{}", value.to_expanded_string("", &config))
                            .map_err(&from_io_error)?,
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        );

        Ok(PipelineData::byte_stream(output, metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Join a list of strings with newlines",
                example: "['first' 'second' 'third'] | lines join",
                result: Some(Value::test_string("first\nsecond\nthird")),
            },
            Example {
                description: "Round-trip: split and join",
                example: r#""one\ntwo\nthree" | lines | lines join"#,
                result: Some(Value::test_string("one\ntwo\nthree")),
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

        test_examples(LinesJoin {})
    }
}
