use super::PathSubcommandArguments;
#[allow(deprecated)]
use nu_engine::{
    command_prelude::*,
    env::{current_dir_str, current_dir_str_const},
};
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::engine::StateWorkingSet;
use std::path::Path;

struct Arguments {
    strict: bool,
    cwd: String,
    not_follow_symlink: bool,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathExpand;

impl Command for PathExpand {
    fn name(&self) -> &str {
        "path expand"
    }

    fn signature(&self) -> Signature {
        Signature::build("path expand")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .switch(
                "strict",
                "Throw an error if the path could not be expanded",
                Some('s'),
            )
            .switch("no-symlink", "Do not resolve symbolic links", Some('n'))
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Try to expand a path to its absolute form."
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
        #[allow(deprecated)]
        let args = Arguments {
            strict: call.has_flag(engine_state, stack, "strict")?,
            cwd: current_dir_str(engine_state, stack)?,
            not_follow_symlink: call.has_flag(engine_state, stack, "no-symlink")?,
        };
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&expand, &args, value, head),
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
        #[allow(deprecated)]
        let args = Arguments {
            strict: call.has_flag_const(working_set, "strict")?,
            cwd: current_dir_str_const(working_set)?,
            not_follow_symlink: call.has_flag_const(working_set, "no-symlink")?,
        };
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&expand, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Expand an absolute path",
                example: r"'C:\Users\joe\foo\..\bar' | path expand",
                result: Some(Value::test_string(r"C:\Users\joe\bar")),
            },
            Example {
                description: "Expand a relative path",
                example: r"'foo\..\bar' | path expand",
                result: None,
            },
            Example {
                description: "Expand a list of paths",
                example: r"[ C:\foo\..\bar, C:\foo\..\baz ] | path expand",
                result: Some(Value::test_list(vec![
                    Value::test_string(r"C:\bar"),
                    Value::test_string(r"C:\baz"),
                ])),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Expand an absolute path",
                example: "'/home/joe/foo/../bar' | path expand",
                result: Some(Value::test_string("/home/joe/bar")),
            },
            Example {
                description: "Expand a relative path",
                example: "'foo/../bar' | path expand",
                result: None,
            },
            Example {
                description: "Expand a list of paths",
                example: "[ /foo/../bar, /foo/../baz ] | path expand",
                result: Some(Value::test_list(vec![
                    Value::test_string("/bar"),
                    Value::test_string("/baz"),
                ])),
            },
        ]
    }
}

fn expand(path: &Path, span: Span, args: &Arguments) -> Value {
    if args.strict {
        match canonicalize_with(path, &args.cwd) {
            Ok(p) => {
                if args.not_follow_symlink {
                    Value::string(
                        expand_path_with(path, &args.cwd, true).to_string_lossy(),
                        span,
                    )
                } else {
                    Value::string(p.to_string_lossy(), span)
                }
            }
            Err(_) => Value::error(
                ShellError::GenericError {
                    error: "Could not expand path".into(),
                    msg: "could not be expanded (path might not exist, non-final \
                            component is not a directory, or other cause)"
                        .into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                },
                span,
            ),
        }
    } else if args.not_follow_symlink {
        Value::string(
            expand_path_with(path, &args.cwd, true).to_string_lossy(),
            span,
        )
    } else {
        canonicalize_with(path, &args.cwd)
            .map(|p| Value::string(p.to_string_lossy(), span))
            .unwrap_or_else(|_| {
                Value::string(
                    expand_path_with(path, &args.cwd, true).to_string_lossy(),
                    span,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathExpand {})
    }
}
