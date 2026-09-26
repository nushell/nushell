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
somewhere else than the textual path suggests. When --walk-up has to add `..`,
names are compared with exact case, even on Windows and macOS."
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
        Ok(p) => Value::string(p.to_string_lossy(), span),
        Err(Refusal::DifferentRoots) => Value::error(
            GenericError::new(
                String::from("Cannot express the input path relative to the argument path."),
                "the input path and the argument path have different roots",
                span,
            )
            .with_help("Both paths need to be absolute, or both relative, and on the same drive.")
            .into(),
            span,
        ),
        Err(Refusal::ParentDirInBase) => Value::error(
            GenericError::new(
                String::from("Cannot walk up from the argument path to the input path."),
                "this path has `..` after the part it shares with the input path",
                args.path.span,
            )
            .with_help("A `..` cannot be walked back without the filesystem. Try `path expand` on the argument path first.")
            .into(),
            span,
        ),
        Err(Refusal::NotParent) => {
            let mut error = GenericError::new(
                String::from("The argument path is not a parent of the input path."),
                "prefix not found",
                span,
            );
            // Suggest the flag only when it would give a result.
            if relative_path(&lhs, &rhs, true).is_ok() {
                error = error.with_help("Use --walk-up to allow `..` components in the result.");
            }
            Value::error(error.into(), span)
        }
    }
}

/// Why [`relative_path`] could not express one path relative to another.
#[derive(Debug, PartialEq)]
enum Refusal {
    /// Without `walk_up`, the base is not a parent of the path.
    NotParent,
    /// One path is absolute and the other relative, or they are on different
    /// Windows drives.
    DifferentRoots,
    /// The base has a `..` after the shared part.
    ParentDirInBase,
}

/// Expresses `path` relative to `base` by comparing their components as text.
///
/// Skips the components both paths share, then returns the rest of `path`.
/// If `base` has components left, refuses with [`Refusal::NotParent`], unless
/// `walk_up` is set: then each of them becomes a `..`. Walking up is refused
/// when a leftover `base` component is a `..`, which cannot be undone without
/// the filesystem, or a root or a Windows prefix. With or without `walk_up`,
/// the result is refused when the rest of `path` starts at a root or a prefix:
/// `base` had none to match it, so the two paths have different roots, and
/// pushed onto a `PathBuf` it would replace the result instead of extending it.
/// This covers a `base` of `.`, which has no components once the `.` is skipped.
///
/// Names are compared ignoring case on systems whose filesystems are usually
/// case-insensitive, but not when walking up. The OS is only a guess about the
/// actual volume, and when walking up a wrong guess would change how many `..`
/// are emitted and land in another directory. Exact names always give a
/// correct result, at worst a longer one. A pair that needs no `..` gets the
/// same answer with or without `walk_up`, so the flag only adds results.
///
/// A leading `.` is skipped on both sides, so `./a` and `a` both mean the same
/// place relative to the current directory. `components()` yields `.` only as
/// the first component, so none is left after this.
fn relative_path(path: &Path, base: &Path, walk_up: bool) -> Result<PathBuf, Refusal> {
    if walk_up && let Ok(relative) = relative_path(path, base, false) {
        return Ok(relative);
    }
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

    if !walk_up && base_rest.peek().is_some() {
        return Err(Refusal::NotParent);
    }
    let mut relative: PathBuf = base_rest
        .map(|c| match c {
            Component::Normal(_) => Ok(Component::ParentDir),
            Component::ParentDir => Err(Refusal::ParentDirInBase),
            _ => Err(Refusal::DifferentRoots),
        })
        .collect::<Result<_, _>>()?;
    if matches!(
        path_rest.peek(),
        Some(Component::RootDir | Component::Prefix(_))
    ) {
        return Err(Refusal::DifferentRoots);
    }

    relative.extend(path_rest);
    Ok(relative)
}

/// Compares two path components. With `fold_case`, names are compared ignoring
/// case on systems whose filesystems are typically case-insensitive.
fn components_eq(a: &Component, b: &Component, fold_case: bool) -> bool {
    match (a, b) {
        (Component::Normal(a), Component::Normal(b))
            if fold_case && is_case_insensitive_filesystem() =>
        {
            // Names that are not valid UTF-8 are compared as they are, so that
            // two different such names are not both turned into U+FFFD.
            a == b
                || a.to_str()
                    .zip(b.to_str())
                    .is_some_and(|(a, b)| a.to_lowercase() == b.to_lowercase())
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
    use nu_protocol::shell_error::ErrorSite;

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

    fn walk_up(path: &str, base: &str) -> Result<PathBuf, Refusal> {
        relative_path(Path::new(path), Path::new(base), true)
    }

    #[test]
    fn walk_up_emits_parent_components() {
        assert_eq!(walk_up("/a/b/c", "/a/d/e"), Ok(PathBuf::from("../../b/c")));
        assert_eq!(walk_up("/a", "/a/b/c"), Ok(PathBuf::from("../..")));
        assert_eq!(walk_up("/x", "/a/b/c"), Ok(PathBuf::from("../../../x")));
        assert_eq!(walk_up("a/b", "c"), Ok(PathBuf::from("../a/b")));
        assert_eq!(walk_up("/a/b", "/a/b"), Ok(PathBuf::new()));
    }

    #[test]
    fn walk_up_refuses_what_text_cannot_answer() {
        // absolute and relative mixed: a pushed root would silently replace the `..` chain
        assert_eq!(walk_up("/a/b", "a"), Err(Refusal::DifferentRoots));
        assert_eq!(walk_up("a/b", "/a"), Err(Refusal::DifferentRoots));
        // `.` has no components once skipped, so nothing matches the root
        assert_eq!(walk_up("/a", "."), Err(Refusal::DifferentRoots));
        assert_eq!(
            relative_path(Path::new("/a"), Path::new("."), false),
            Err(Refusal::DifferentRoots)
        );
        // a `..` left in the base cannot be undone without the filesystem
        assert_eq!(walk_up("a/b", "a/../c"), Err(Refusal::ParentDirInBase));
    }

    #[test]
    fn leading_cur_dir_is_ignored() {
        let expected = Ok(PathBuf::from("../a"));
        assert_eq!(walk_up("./a", "b"), expected);
        assert_eq!(walk_up("a", "./b"), expected);
        assert_eq!(walk_up("./a", "./b"), expected);
        assert_eq!(
            relative_path(Path::new("./a/b"), Path::new("a"), false),
            Ok(PathBuf::from("b"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn walk_up_refuses_other_drive() {
        assert_eq!(walk_up(r"C:\a", r"D:\a"), Err(Refusal::DifferentRoots));
    }

    #[test]
    fn walk_up_compares_names_exactly_only_when_walking() {
        // a pair the plain command answers keeps that answer
        let expected = if is_case_insensitive_filesystem() {
            ""
        } else {
            "../etc"
        };
        assert_eq!(walk_up("/etc", "/Etc"), Ok(PathBuf::from(expected)));
        // `Foo` and `foo` may be different directories even on Windows or macOS
        assert_eq!(
            walk_up("/v/Foo", "/v/foo/bar"),
            Ok(PathBuf::from("../../Foo"))
        );
    }

    /// Returns the span that the error label of `value` points at.
    fn error_label_span(value: Value) -> Span {
        match value {
            Value::Error { error, .. } => match *error {
                ShellError::Generic(GenericError {
                    site: ErrorSite::Span(span),
                    ..
                }) => span,
                other => panic!("expected a generic error with a span, got {other:?}"),
            },
            other => panic!("expected an error, got {other:?}"),
        }
    }

    /// Returns the help of the error in `value`.
    fn error_help(value: Value) -> Option<String> {
        match value {
            Value::Error { error, .. } => match *error {
                ShellError::Generic(GenericError { help, .. }) => help.map(|h| h.into_owned()),
                other => panic!("expected a generic error, got {other:?}"),
            },
            other => panic!("expected an error, got {other:?}"),
        }
    }

    #[test]
    fn walk_up_hint_only_when_it_would_work() {
        let args = |path: &str| Arguments {
            path: Spanned {
                item: path.to_string(),
                span: Span::test_data(),
            },
            walk_up: false,
        };
        let help = |path: &str, base: &str| {
            error_help(relative_to(Path::new(path), Span::test_data(), &args(base)))
        };

        assert!(help("/a/b", "/a/c").is_some());
        assert_eq!(help("/a/b", "a"), None);
        assert_eq!(help("a/b", "a/../c"), None);
    }

    #[test]
    fn walk_up_errors_point_at_what_to_change() {
        let input_span = Span::new(0, 5);
        let arg_span = Span::new(10, 20);
        let args = |path: &str| Arguments {
            path: Spanned {
                item: path.to_string(),
                span: arg_span,
            },
            walk_up: true,
        };

        let result = relative_to(Path::new("a/b"), input_span, &args("a/../c"));
        assert_eq!(error_label_span(result), arg_span);

        let result = relative_to(Path::new("/a/b"), input_span, &args("a"));
        assert_eq!(error_label_span(result), input_span);
    }

    #[cfg(unix)]
    #[test]
    fn different_non_utf8_names_are_not_equal() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let a = Component::Normal(OsStr::from_bytes(b"\xff"));
        let b = Component::Normal(OsStr::from_bytes(b"\xfe"));
        assert!(!components_eq(&a, &b, true));
        assert!(components_eq(&a, &a, true));
    }

    #[test]
    fn without_walk_up_argument_must_be_parent() {
        assert_eq!(
            relative_path(Path::new("/a/b/c"), Path::new("/a/d"), false),
            Err(Refusal::NotParent)
        );
    }
}
