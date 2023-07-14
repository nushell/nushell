use std::path::Path;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    engine::Command, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
    replace: Option<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {
    fn get_columns(&self) -> Option<Vec<String>> {
        self.columns.clone()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
                "columns",
                SyntaxShape::Table(vec![]),
                "For a record or table input, convert strings in the given columns to their basename",
                Some('c'),
            )
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with basename replaced by this string",
                Some('r'),
            )
    }

    fn usage(&self) -> &str {
        "Get the final component of a path."
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
            columns: call.get_flag(engine_state, stack, "columns")?,
            replace: call.get_flag(engine_state, stack, "replace")?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&get_basename, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "'C:\\Users\\joe\\test.txt' | path basename",
                result: Some(Value::test_string("test.txt")),
            },
            Example {
                description: "Get basename of a path in a column",
                example: "ls .. | path basename -c [ name ]",
                result: None,
            },
            Example {
                description: "Get basename of a path in a column",
                example: "[[name];[C:\\Users\\Joe]] | path basename -c [ name ]",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["name".to_string()],
                        vals: vec![Value::test_string("Joe")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Replace basename of a path",
                example: "'C:\\Users\\joe\\test.txt' | path basename -r 'spam.png'",
                result: Some(Value::test_string("C:\\Users\\joe\\spam.png")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "'/home/joe/test.txt' | path basename",
                result: Some(Value::test_string("test.txt")),
            },
            Example {
                description: "Get basename of a path by column",
                example: "[[name];[/home/joe]] | path basename -c [ name ]",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["name".to_string()],
                        vals: vec![Value::test_string("joe")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Replace basename of a path",
                example: "'/home/joe/test.txt' | path basename -r 'spam.png'",
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

        test_examples(SubCommand {})
    }
}
