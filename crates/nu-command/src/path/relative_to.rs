use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_path::expand_to_real_path;
use nu_protocol::{engine::StateWorkingSet, shell_error::generic::GenericError};
use std::path::{Component, Path, PathBuf};

struct Arguments {
    path: Spanned<String>,
    walk_up: bool,
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
                "Path to express the input path relative to.",
            )
            .switch(
                "walk-up",
                "Allow `..` components when the argument path is not a parent of the input path.",
                None,
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Express a path as relative to another path."
    }

    fn extra_description(&self) -> &str {
        "Can be used only when the input and the argument paths are either both
absolute or both relative. Without --walk-up, the argument path needs to be a
parent of the input path.

The paths are compared as text, without touching the filesystem. With --walk-up,
each `..` in the result stands for the textual parent of the argument path, so
if the argument path goes through a symbolic link, the result may point
somewhere else than the textual path suggests. Also with --walk-up, names are
compared with exact case, even on Windows and macOS."
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
            walk_up: call.has_flag(engine_state, stack, "walk-up")?,
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
            walk_up: call.has_flag_const(working_set, stack, "walk-up")?,
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
            Example {
                description: "Walk up with `..` when the argument path is not a parent of the input path.",
                example: if cfg!(windows) {
                    r"'C:\Users\viking' | path relative-to 'C:\Users\spam\eggs' --walk-up"
                } else {
                    "'/home/viking' | path relative-to '/home/spam/eggs' --walk-up"
                },
                result: Some(Value::test_string(if cfg!(windows) {
                    r"..\..\viking"
                } else {
                    "../../viking"
                })),
            },
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let lhs = expand_to_real_path(path);
    let rhs = expand_to_real_path(&args.path.item);

    match relative_path(&lhs, &rhs, args.walk_up) {
        Some(p) => Value::string(p.to_string_lossy(), span),
        None if args.walk_up => Value::error(
            GenericError::new(
                String::from("Cannot walk up from the argument path to the input path."),
                "the paths have different roots, or the argument path has `.` or `..` after the shared part",
                span,
            )
            .into(),
            span,
        ),
        None => Value::error(
            GenericError::new(
                String::from("The argument path is not a parent of the input path."),
                "prefix not found",
                span,
            )
            .with_help("Use --walk-up to allow `..` components in the result.")
            .into(),
            span,
        ),
    }
}

/// Expresses `path` relative to `base` by comparing their components as text.
///
/// Skips the components both paths share, then returns the rest of `path`.
/// If `base` has components left, returns `None`, unless `walk_up` is set:
/// then each of them becomes a `..`. Walking up is refused (`None`) when a
/// leftover `base` component is not a plain name (a `..` there cannot be undone
/// without the filesystem) or when the rest of `path` starts at a root or a
/// Windows prefix: pushed onto a `PathBuf`, they would replace the `..` chain
/// instead of extending it.
///
/// Names are compared ignoring case on systems whose filesystems are usually
/// case-insensitive, but only without `walk_up`. The OS is only a guess about
/// the actual volume, and with `walk_up` a wrong guess would change how many
/// `..` are emitted and land in another directory. Exact names always give a
/// correct result, at worst a longer one.
///
/// A leading `.` is skipped on both sides, so `./a` and `a` both mean the same
/// place relative to the current directory. `components()` yields `.` only as
/// the first component, so none is left after this.
fn relative_path(path: &Path, base: &Path, walk_up: bool) -> Option<PathBuf> {
    let mut path_rest = path.components().peekable();
    let mut base_rest = base.components().peekable();
    path_rest.next_if_eq(&Component::CurDir);
    base_rest.next_if_eq(&Component::CurDir);
    while let (Some(p), Some(b)) = (path_rest.peek(), base_rest.peek())
        && components_eq(p, b, !walk_up)
    {
        path_rest.next();
        base_rest.next();
    }

    let mut relative: PathBuf = base_rest
        .map(|c| match c {
            Component::Normal(_) if walk_up => Some(Component::ParentDir),
            _ => None,
        })
        .collect::<Option<_>>()?;
    if !relative.as_os_str().is_empty()
        && matches!(
            path_rest.peek(),
            Some(Component::RootDir | Component::Prefix(_))
        )
    {
        return None;
    }

    relative.extend(path_rest);
    Some(relative)
}

/// Compares two path components. With `fold_case`, names are compared ignoring
/// case on systems whose filesystems are typically case-insensitive.
fn components_eq(a: &Component, b: &Component, fold_case: bool) -> bool {
    match (a, b) {
        (Component::Normal(a), Component::Normal(b))
            if fold_case && is_case_insensitive_filesystem() =>
        {
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
            walk_up: false,
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
            walk_up: false,
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
            walk_up: false,
        };

        let result = relative_to(Path::new("/home/user"), Span::test_data(), &args);

        // This should fail on all filesystems since paths are truly different
        match result {
            Value::Error { .. } => {}
            _ => panic!("Expected error for truly different paths"),
        }
    }

    fn walk_up(path: &str, base: &str) -> Option<PathBuf> {
        relative_path(Path::new(path), Path::new(base), true)
    }

    #[test]
    fn walk_up_emits_parent_components() {
        assert_eq!(
            walk_up("/a/b/c", "/a/d/e"),
            Some(PathBuf::from("../../b/c"))
        );
        assert_eq!(walk_up("/a", "/a/b/c"), Some(PathBuf::from("../..")));
        assert_eq!(walk_up("/x", "/a/b/c"), Some(PathBuf::from("../../../x")));
        assert_eq!(walk_up("a/b", "c"), Some(PathBuf::from("../a/b")));
        assert_eq!(walk_up("/a/b", "/a/b"), Some(PathBuf::new()));
    }

    #[test]
    fn walk_up_refuses_what_text_cannot_answer() {
        // absolute and relative mixed: a pushed root would silently replace the `..` chain
        assert_eq!(walk_up("/a/b", "a"), None);
        assert_eq!(walk_up("a/b", "/a"), None);
        // a `..` left in the base cannot be undone without the filesystem
        assert_eq!(walk_up("a/b", "a/../c"), None);
    }

    #[test]
    fn leading_cur_dir_is_ignored() {
        let expected = Some(PathBuf::from("../a"));
        assert_eq!(walk_up("./a", "b"), expected);
        assert_eq!(walk_up("a", "./b"), expected);
        assert_eq!(walk_up("./a", "./b"), expected);
        assert_eq!(
            relative_path(Path::new("./a/b"), Path::new("a"), false),
            Some(PathBuf::from("b"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn walk_up_refuses_other_drive() {
        assert_eq!(walk_up(r"C:\a", r"D:\a"), None);
    }

    #[test]
    fn walk_up_compares_names_exactly() {
        // `Foo` and `foo` may be different directories even on Windows or macOS
        assert_eq!(walk_up("/etc", "/Etc"), Some(PathBuf::from("../etc")));
        assert_eq!(
            walk_up("/v/Foo", "/v/foo/bar"),
            Some(PathBuf::from("../../Foo"))
        );
    }

    #[test]
    fn without_walk_up_argument_must_be_parent() {
        assert_eq!(
            relative_path(Path::new("/a/b/c"), Path::new("/a/d"), false),
            None
        );
    }
}
