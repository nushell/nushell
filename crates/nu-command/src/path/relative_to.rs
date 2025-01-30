use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_path::expand_to_real_path;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Path, PathBuf};

struct Arguments {
    path: Spanned<String>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path relative-to"
    }

    fn signature(&self) -> Signature {
        Signature::build("path relative-to")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .required(
                "path",
                SyntaxShape::String,
                "Parent shared with the input path.",
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Express a path as relative to another path."
    }

    fn extra_description(&self) -> &str {
        r#"Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path."#
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
            path: call.req(engine_state, stack, 0)?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
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
            path: call.req_const(working_set, 0)?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
            working_set.permanent().signals(),
        )
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
                description: "Find a relative path from absolute paths in list",
                example: r"[ /home/viking, /home/spam ] | path relative-to '/home'",
                result: Some(Value::test_list(vec![
                    Value::test_string("viking"),
                    Value::test_string("spam"),
                ])),
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'",
                result: Some(Value::test_string(r"spam")),
            },
            Example {
                description: "Find a relative path that requires parent directory symbols",
                example: r"'a/b/c' | path relative-to 'a/d/e'",

                result: Some(Value::test_string(r"../../b/c")),
            },
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let child = expand_to_real_path(path);
    let parent = expand_to_real_path(&args.path.item);

    let common: PathBuf = child
        .iter()
        .zip(parent.iter())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x)
        .collect();

    let differing_parent = match parent.strip_prefix(&common) {
        Ok(p) => p,
        Err(_) => {
            return Value::error(
                ShellError::IncorrectValue {
                    msg: "Unable to strip common prefix from parent".into(),
                    val_span: span,
                    call_span: span,
                },
                span,
            )
        }
    };

    let differing_child = match child.strip_prefix(&common) {
        Ok(p) => p,
        Err(_) => {
            return Value::error(
                ShellError::IncorrectValue {
                    msg: "Unable to strip common prefix from child".into(),
                    val_span: span,
                    call_span: span,
                },
                span,
            )
        }
    };

    let mut path = PathBuf::new();
    differing_parent.iter().for_each(|_| path.push(".."));

    path.push(differing_child);

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
