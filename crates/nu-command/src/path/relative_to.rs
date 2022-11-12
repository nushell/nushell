use std::path::Path;

use nu_engine::CallExt;
use nu_path::expand_to_real_path;
use nu_protocol::{
    engine::Command, Example, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    path: Spanned<String>,
    columns: Option<Vec<String>>,
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
        "path relative-to"
    }

    fn signature(&self) -> Signature {
        Signature::build("path relative-to")
            .input_output_types(vec![(Type::String, Type::String)])
            .required(
                "path",
                SyntaxShape::String,
                "Parent shared with the input path",
            )
            .named(
                "columns",
                SyntaxShape::Table,
                "For a record or table input, convert strings at the given columns",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Express a path as relative to another path."
    }

    fn extra_usage(&self) -> &str {
        r#"Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path."#
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            path: call.req(engine_state, stack, 0)?,
            columns: call.get_flag(engine_state, stack, "columns")?,
        };

        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'C:\Users\viking' | path relative-to 'C:\Users'",
                result: Some(Value::test_string(r"viking")),
            },
            Example {
                description: "Find a relative path from two absolute paths in a column",
                example: "ls ~ | path relative-to ~ -c [ name ]",
                result: None,
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs\bacon\sausage\spam' | path relative-to 'eggs\bacon\sausage'",
                result: Some(Value::test_string(r"spam")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'/home/viking' | path relative-to '/home'",
                result: Some(Value::test_string(r"viking")),
            },
            Example {
                description: "Find a relative path from two absolute paths in a column",
                example: "ls ~ | path relative-to ~ -c [ name ]",
                result: None,
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'",
                result: Some(Value::test_string(r"spam")),
            },
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let lhs = expand_to_real_path(path);
    let rhs = expand_to_real_path(&args.path.item);
    match lhs.strip_prefix(&rhs) {
        Ok(p) => Value::string(p.to_string_lossy(), span),
        Err(e) => Value::Error {
            error: ShellError::CantConvert(e.to_string(), "string".into(), span, None),
        },
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
