use std::path::Path;

use nu_engine::CallExt;
use nu_protocol::{engine::Command, Example, Signature, Span, Spanned, SyntaxShape, Type, Value};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
    replace: Option<Spanned<String>>,
    num_levels: Option<i64>,
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
        "path dirname"
    }

    fn signature(&self) -> Signature {
        Signature::build("path dirname")
            .input_output_types(vec![(Type::String, Type::String)])
            .named(
                "columns",
                SyntaxShape::Table,
                "For a record or table input, convert strings at the given columns to their dirname",
                Some('c'),
            )
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with dirname replaced by this string",
                Some('r'),
            )
            .named(
                "num-levels",
                SyntaxShape::Int,
                "Number of directories to walk up",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Get the parent directory of a path"
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let args = Arguments {
            columns: call.get_flag(engine_state, stack, "columns")?,
            replace: call.get_flag(engine_state, stack, "replace")?,
            num_levels: call.get_flag(engine_state, stack, "num-levels")?,
        };

        input.map(
            move |value| super::operate(&get_dirname, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname",
                result: Some(Value::test_string("C:\\Users\\joe\\code")),
            },
            Example {
                description: "Get dirname of a path in a column",
                example: "ls ('.' | path expand) | path dirname -c [ name ]",
                result: None,
            },
            Example {
                description: "Walk up two levels",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2",
                result: Some(Value::test_string("C:\\Users\\joe")),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example:
                    "'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2 -r C:\\Users\\viking",
                result: Some(Value::test_string("C:\\Users\\viking\\code\\test.txt")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'/home/joe/code/test.txt' | path dirname",
                result: Some(Value::test_string("/home/joe/code")),
            },
            Example {
                description: "Get dirname of a path in a column",
                example: "ls ('.' | path expand) | path dirname -c [ name ]",
                result: None,
            },
            Example {
                description: "Walk up two levels",
                example: "'/home/joe/code/test.txt' | path dirname -n 2",
                result: Some(Value::test_string("/home/joe")),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example: "'/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking",
                result: Some(Value::test_string("/home/viking/code/test.txt")),
            },
        ]
    }
}

fn get_dirname(path: &Path, span: Span, args: &Arguments) -> Value {
    let num_levels = args.num_levels.as_ref().map_or(1, |val| *val);

    let mut dirname = path;
    let mut reached_top = false;
    for _ in 0..num_levels {
        dirname = dirname.parent().unwrap_or_else(|| {
            reached_top = true;
            dirname
        });
        if reached_top {
            break;
        }
    }

    let path = match args.replace {
        Some(ref newdir) => {
            let remainder = path.strip_prefix(dirname).unwrap_or(dirname);
            if !remainder.as_os_str().is_empty() {
                Path::new(&newdir.item).join(remainder)
            } else {
                Path::new(&newdir.item).to_path_buf()
            }
        }
        None => dirname.to_path_buf(),
    };

    Value::string(path.to_string_lossy(), span)
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
