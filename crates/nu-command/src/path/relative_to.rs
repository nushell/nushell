use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_path::expand_to_real_path;
use nu_protocol::engine::StateWorkingSet;
use std::path::Path;

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

    fn usage(&self) -> &str {
        "Express a path as relative to another path."
    }

    fn extra_usage(&self) -> &str {
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
            engine_state.ctrlc.clone(),
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
            working_set.permanent().ctrlc.clone(),
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
                description: "Find a relative path from absolute paths in list",
                example: r"[ C:\Users\viking, C:\Users\spam ] | path relative-to C:\Users",
                result: Some(Value::test_list(vec![
                    Value::test_string("viking"),
                    Value::test_string("spam"),
                ])),
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
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let lhs = expand_to_real_path(path);
    let rhs = expand_to_real_path(&args.path.item);
    match lhs.strip_prefix(&rhs) {
        Ok(p) => Value::string(p.to_string_lossy(), span),
        Err(e) => Value::error(
            ShellError::CantConvert {
                to_type: e.to_string(),
                from_type: "string".into(),
                span,
                help: None,
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
