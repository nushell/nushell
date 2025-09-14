use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use std::path::Path;

struct Arguments {
    replace: Option<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathBasename;

impl Command for PathBasename {
    fn name(&self) -> &str {
        "path basename"
    }

    fn signature(&self) -> Signature {
        Signature::build("path basename")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with basename replaced by this string",
                Some('r'),
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Get the final component of a path."
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
        let head = call.head;
        let args = Arguments {
            replace: call.get_flag(engine_state, stack, "replace")?,
        };

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&get_basename, &args, value, head),
            engine_state.signals(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            replace: call.get_flag_const(working_set, "replace")?,
        };

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&get_basename, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "'C:\\Users\\joe\\test.txt' | path basename",
                result: Some(Value::test_string("test.txt")),
            },
            Example {
                description: "Get basename of a list of paths",
                example: r"[ C:\Users\joe, C:\Users\doe ] | path basename",
                result: Some(Value::test_list(vec![
                    Value::test_string("joe"),
                    Value::test_string("doe"),
                ])),
            },
            Example {
                description: "Replace basename of a path",
                example: "'C:\\Users\\joe\\test.txt' | path basename --replace 'spam.png'",
                result: Some(Value::test_string("C:\\Users\\joe\\spam.png")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "'/home/joe/test.txt' | path basename",
                result: Some(Value::test_string("test.txt")),
            },
            Example {
                description: "Get basename of a list of paths",
                example: "[ /home/joe, /home/doe ] | path basename",
                result: Some(Value::test_list(vec![
                    Value::test_string("joe"),
                    Value::test_string("doe"),
                ])),
            },
            Example {
                description: "Replace basename of a path",
                example: "'/home/joe/test.txt' | path basename --replace 'spam.png'",
                result: Some(Value::test_string("/home/joe/spam.png")),
            },
        ]
    }
}

fn get_basename(path: &Path, span: Span, args: &Arguments) -> Value {
    match &args.replace {
        Some(r) => Value::string(path.with_file_name(r.item.clone()).to_string_lossy(), span),
        None => Value::string(
            match path.file_name() {
                Some(n) => n.to_string_lossy(),
                None => "".into(),
            },
            span,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathBasename {})
    }
}
