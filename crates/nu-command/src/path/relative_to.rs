use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_path::expand_to_real_path;
use nu_protocol::{engine::StateWorkingSet, shell_error::generic::GenericError};
use std::path::{Component, Path, PathBuf};

struct Arguments {
    path: Spanned<String>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathRelativeTo;

impl Command for PathRelativeTo {
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
        "Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path."
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
        if let PipelineData::Empty = input {
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            path: call.req_const(working_set, stack, 0)?,
        };

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths.",
                example: if cfg!(windows) {
                    r"'C:\Users\viking' | path relative-to 'C:\Users'"
                } else {
                    "'/home/viking' | path relative-to '/home'"
                },
                result: Some(Value::test_string("viking")),
            },
            Example {
                description: "Find a relative path from absolute paths in list.",
                example: if cfg!(windows) {
                    r"[ C:\Users\viking, C:\Users\spam ] | path relative-to C:\Users"
                } else {
                    "[ /home/viking, /home/spam ] | path relative-to '/home'"
                },
                result: Some(Value::test_list(vec![
                    Value::test_string("viking"),
                    Value::test_string("spam"),
                ])),
            },
            Example {
                description: "Find a relative path from two relative paths.",
                example: if cfg!(windows) {
                    r"'eggs\bacon\sausage\spam' | path relative-to 'eggs\bacon\sausage'"
                } else {
                    "'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'"
                },
                result: Some(Value::test_string("spam")),
            },
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let lhs = expand_to_real_path(path);
    let rhs = expand_to_real_path(&args.path.item);

    match relative_path(&lhs, &rhs) {
        Some(p) => Value::string(p.to_string_lossy(), span),
        None => Value::error(
            GenericError::new(
                String::from("The argument path is not a parent of the input path."),
                "prefix not found",
                span,
            )
            .into(),
            span,
        ),
    }
}

/// Expresses `path` relative to `base` by comparing their components as text.
///
/// Skips the components both paths share, then returns the rest of `path`.
/// If `base` has components left, returns `None`.
fn relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut path_rest = path.components().peekable();
    let mut base_rest = base.components().peekable();
    while let (Some(p), Some(b)) = (path_rest.peek(), base_rest.peek())
        && components_eq(p, b)
    {
        path_rest.next();
        base_rest.next();
    }

    if base_rest.next().is_some() {
        return None;
    }

    Some(path_rest.collect())
}

/// Compares two path components, ignoring case of names on case-insensitive filesystems.
fn components_eq(a: &Component, b: &Component) -> bool {
    match (a, b) {
        (Component::Normal(a), Component::Normal(b)) if is_case_insensitive_filesystem() => {
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        }
        _ => a == b,
    }
}

/// Check if the current filesystem is typically case-insensitive
fn is_case_insensitive_filesystem() -> bool {
    // Windows and macOS typically have case-insensitive filesystems
    cfg!(any(target_os = "windows", target_os = "macos"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(PathRelativeTo)
    }

    #[test]
    fn test_case_insensitive_filesystem() {
        use nu_protocol::{Span, Value};
        use std::path::Path;

        let args = Arguments {
            path: Spanned {
                item: "/Etc".to_string(),
                span: Span::test_data(),
            },
        };

        let result = relative_to(Path::new("/etc"), Span::test_data(), &args);

        // On case-insensitive filesystems (Windows, macOS), this should work
        // On case-sensitive filesystems (Linux, FreeBSD), this should fail
        if is_case_insensitive_filesystem() {
            match result {
                Value::String { val, .. } => {
                    assert_eq!(val, "");
                }
                _ => panic!("Expected string result on case-insensitive filesystem"),
            }
        } else {
            match result {
                Value::Error { .. } => {
                    // Expected on case-sensitive filesystems
                }
                _ => panic!("Expected error on case-sensitive filesystem"),
            }
        }
    }

    #[test]
    fn test_case_insensitive_with_subpath() {
        use nu_protocol::{Span, Value};
        use std::path::Path;

        let args = Arguments {
            path: Spanned {
                item: "/Home/User".to_string(),
                span: Span::test_data(),
            },
        };

        let result = relative_to(Path::new("/home/user/documents"), Span::test_data(), &args);

        if is_case_insensitive_filesystem() {
            match result {
                Value::String { val, .. } => {
                    assert_eq!(val, "documents");
                }
                _ => panic!("Expected string result on case-insensitive filesystem"),
            }
        } else {
            match result {
                Value::Error { .. } => {
                    // Expected on case-sensitive filesystems
                }
                _ => panic!("Expected error on case-sensitive filesystem"),
            }
        }
    }

    #[test]
    fn test_truly_different_paths() {
        use nu_protocol::{Span, Value};
        use std::path::Path;

        let args = Arguments {
            path: Spanned {
                item: "/Different/Path".to_string(),
                span: Span::test_data(),
            },
        };

        let result = relative_to(Path::new("/home/user"), Span::test_data(), &args);

        // This should fail on all filesystems since paths are truly different
        match result {
            Value::Error { .. } => {}
            _ => panic!("Expected error for truly different paths"),
        }
    }
}
