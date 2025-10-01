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
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            path: call.req_const(working_set, 0)?,
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

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
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
    fn examples(&self) -> Vec<Example<'_>> {
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
        Err(e) => {
            // On case-insensitive filesystems, try case-insensitive comparison
            if is_case_insensitive_filesystem()
                && let Some(relative_path) = try_case_insensitive_strip_prefix(&lhs, &rhs)
            {
                return Value::string(relative_path.to_string_lossy(), span);
            }

            Value::error(
                ShellError::CantConvert {
                    to_type: e.to_string(),
                    from_type: "string".into(),
                    span,
                    help: None,
                },
                span,
            )
        }
    }
}

/// Check if the current filesystem is typically case-insensitive
fn is_case_insensitive_filesystem() -> bool {
    // Windows and macOS typically have case-insensitive filesystems
    cfg!(any(target_os = "windows", target_os = "macos"))
}

/// Try to strip prefix in a case-insensitive manner
fn try_case_insensitive_strip_prefix(lhs: &Path, rhs: &Path) -> Option<std::path::PathBuf> {
    let mut lhs_components = lhs.components();
    let mut rhs_components = rhs.components();

    // Compare components case-insensitively
    loop {
        match (lhs_components.next(), rhs_components.next()) {
            (Some(lhs_comp), Some(rhs_comp)) => {
                match (lhs_comp, rhs_comp) {
                    (
                        std::path::Component::Normal(lhs_name),
                        std::path::Component::Normal(rhs_name),
                    ) => {
                        if lhs_name.to_string_lossy().to_lowercase()
                            != rhs_name.to_string_lossy().to_lowercase()
                        {
                            return None;
                        }
                    }
                    // Non-Normal components must match exactly
                    _ if lhs_comp != rhs_comp => {
                        return None;
                    }
                    _ => {}
                }
            }
            (Some(lhs_comp), None) => {
                // rhs is fully consumed, but lhs has more components
                // This means rhs is a prefix of lhs, collect remaining lhs components
                let mut result = std::path::PathBuf::new();
                // Add the current lhs component that wasn't matched
                result.push(lhs_comp);
                // Add all remaining lhs components
                for component in lhs_components {
                    result.push(component);
                }
                return Some(result);
            }
            (None, Some(_)) => {
                // lhs is shorter than rhs, so rhs cannot be a prefix of lhs
                return None;
            }
            (None, None) => {
                // Both paths have the same components, relative path is empty
                return Some(std::path::PathBuf::new());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathRelativeTo {})
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
