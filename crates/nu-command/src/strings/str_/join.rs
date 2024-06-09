use nu_engine::command_prelude::*;

use std::io::Write;

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

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<String> = call.opt(engine_state, stack, 0)?;
        run(engine_state, call, input, separator)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<String> = call.opt_const(working_set, 0)?;
        run(working_set.permanent(), call, input, separator)
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
        ]
    }
}

fn run(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    separator: Option<String>,
) -> Result<PipelineData, ShellError> {
    let config = engine_state.config.clone();

    let span = call.head;

    let metadata = input.metadata();
    let mut iter = input.into_iter();
    let mut first = true;

    let output = ByteStream::from_fn(span, None, ByteStreamType::String, move |buffer| {
        // Write each input to the buffer
        if let Some(value) = iter.next() {
            // Write the separator if this is not the first
            if first {
                first = false;
            } else if let Some(separator) = &separator {
                write!(buffer, "{}", separator)?;
            }

            match value {
                Value::Error { error, .. } => {
                    return Err(*error);
                }
                // Hmm, not sure what we actually want.
                // `to_expanded_string` formats dates as human readable which feels funny.
                Value::Date { val, .. } => write!(buffer, "{val:?}")?,
                value => write!(buffer, "{}", value.to_expanded_string("\n", &config))?,
            }
            Ok(true)
        } else {
            Ok(false)
        }
    });

    Ok(PipelineData::ByteStream(output, metadata))
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
