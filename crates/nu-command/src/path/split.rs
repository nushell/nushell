use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Component, Path};

struct Arguments;

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathSplit;

impl Command for PathSplit {
    fn name(&self) -> &str {
        "path split"
    }

    fn signature(&self) -> Signature {
        Signature::build("path split")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String)))),
                ),
            ])
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Split a path into a list based on the system's path separator."
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments;

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&split, &args, value, head),
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
        let args = Arguments;

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&split, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Split a path into parts.",
                example: r"'C:\Users\viking\spam.txt' | path split",
                result: Some(Value::list(
                    vec![
                        Value::test_string(r"C:\"),
                        Value::test_string("Users"),
                        Value::test_string("viking"),
                        Value::test_string("spam.txt"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split paths in list into parts.",
                example: r"[ C:\Users\viking\spam.txt C:\Users\viking\eggs.txt ] | path split",
                result: Some(Value::list(
                    vec![
                        Value::test_list(vec![
                            Value::test_string(r"C:\"),
                            Value::test_string("Users"),
                            Value::test_string("viking"),
                            Value::test_string("spam.txt"),
                        ]),
                        Value::test_list(vec![
                            Value::test_string(r"C:\"),
                            Value::test_string("Users"),
                            Value::test_string("viking"),
                            Value::test_string("eggs.txt"),
                        ]),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Split a path into parts.",
                example: "'/home/viking/spam.txt' | path split",
                result: Some(Value::list(
                    vec![
                        Value::test_string("/"),
                        Value::test_string("home"),
                        Value::test_string("viking"),
                        Value::test_string("spam.txt"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split paths in list into parts.",
                example: "[ /home/viking/spam.txt /home/viking/eggs.txt ] | path split",
                result: Some(Value::list(
                    vec![
                        Value::test_list(vec![
                            Value::test_string("/"),
                            Value::test_string("home"),
                            Value::test_string("viking"),
                            Value::test_string("spam.txt"),
                        ]),
                        Value::test_list(vec![
                            Value::test_string("/"),
                            Value::test_string("home"),
                            Value::test_string("viking"),
                            Value::test_string("eggs.txt"),
                        ]),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

#[cfg(windows)]
fn split_components(path: &Path) -> Vec<String> {
    let mut components = path.components().peekable();
    let mut parts = Vec::new();

    while let Some(component) = components.next() {
        match component {
            Component::Prefix(_) => {
                let mut prefix = component.as_os_str().to_string_lossy().to_string();
                if matches!(components.peek(), Some(Component::RootDir)) {
                    prefix.push('\\');
                    components.next();
                }
                parts.push(prefix);
            }
            Component::RootDir => {}
            component => parts.push(component.as_os_str().to_string_lossy().to_string()),
        }
    }

    parts
}

fn split(path: &Path, span: Span, _: &Arguments) -> Value {
    #[cfg(windows)]
    let parts = split_components(path);

    #[cfg(not(windows))]
    let parts = path
        .components()
        .filter_map(process_component)
        .collect::<Vec<_>>();

    Value::list(
        parts
            .into_iter()
            .map(|part| Value::string(part, span))
            .collect(),
        span,
    )
}

#[cfg(not(windows))]
fn process_component(comp: Component) -> Option<String> {
    Some(comp.as_os_str().to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(PathSplit)
    }

    #[cfg(windows)]
    #[test]
    fn keeps_drive_relative_prefix_without_root_separator() {
        assert_eq!(split_components(Path::new("C:Desktop")), ["C:", "Desktop"]);
    }

    #[cfg(windows)]
    #[test]
    fn keeps_root_separator_for_absolute_drive_path() {
        assert_eq!(split_components(Path::new(r"C:\\Users")), [r"C:\", "Users"]);
    }

    #[cfg(windows)]
    #[test]
    fn keeps_unc_share_as_rooted_prefix() {
        assert_eq!(
            split_components(Path::new(r"\\server\share\some\path")),
            [r"\\server\share\", "some", "path"]
        );
    }
}
