use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use std::path::Path;

struct Arguments {
    replace: Option<Spanned<String>>,
    num_levels: Option<i64>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathDirname;

impl Command for PathDirname {
    fn name(&self) -> &str {
        "path dirname"
    }

    fn signature(&self) -> Signature {
        Signature::build("path dirname")
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
                "Return original path with dirname replaced by this string",
                Some('r'),
            )
            .named(
                "num-levels",
                SyntaxShape::Int,
                "Number of directories to walk up",
                Some('n'),
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Get the parent directory of a path."
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
            num_levels: call.get_flag(engine_state, stack, "num-levels")?,
        };

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&get_dirname, &args, value, head),
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
            num_levels: call.get_flag_const(working_set, "num-levels")?,
        };

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&get_dirname, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname",
                result: Some(Value::test_string("C:\\Users\\joe\\code")),
            },
            Example {
                description: "Get dirname of a list of paths",
                example: r"[ C:\Users\joe\test.txt, C:\Users\doe\test.txt ] | path dirname",
                result: Some(Value::test_list(vec![
                    Value::test_string(r"C:\Users\joe"),
                    Value::test_string(r"C:\Users\doe"),
                ])),
            },
            Example {
                description: "Walk up two levels",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname --num-levels 2",
                result: Some(Value::test_string("C:\\Users\\joe")),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname --num-levels 2 --replace C:\\Users\\viking",
                result: Some(Value::test_string("C:\\Users\\viking\\code\\test.txt")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'/home/joe/code/test.txt' | path dirname",
                result: Some(Value::test_string("/home/joe/code")),
            },
            Example {
                description: "Get dirname of a list of paths",
                example: "[ /home/joe/test.txt, /home/doe/test.txt ] | path dirname",
                result: Some(Value::test_list(vec![
                    Value::test_string("/home/joe"),
                    Value::test_string("/home/doe"),
                ])),
            },
            Example {
                description: "Walk up two levels",
                example: "'/home/joe/code/test.txt' | path dirname --num-levels 2",
                result: Some(Value::test_string("/home/joe")),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example: "'/home/joe/code/test.txt' | path dirname --num-levels 2 --replace /home/viking",
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

        test_examples(PathDirname {})
    }
}
