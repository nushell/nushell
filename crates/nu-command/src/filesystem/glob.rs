use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals};
use wax::{Glob as WaxGlob, WalkBehavior, WalkEntry};

#[derive(Clone)]
pub struct Glob;

impl Command for Glob {
    fn name(&self) -> &str {
        "glob"
    }

    fn signature(&self) -> Signature {
        Signature::build("glob")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .required("glob", SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::GlobPattern]), "The glob expression.")
            .named(
                "depth",
                SyntaxShape::Int,
                "directory depth to search",
                Some('d'),
            )
            .switch(
                "no-dir",
                "Whether to filter out directories from the returned paths",
                Some('D'),
            )
            .switch(
                "no-file",
                "Whether to filter out files from the returned paths",
                Some('F'),
            )
            .switch(
                "no-symlink",
                "Whether to filter out symlinks from the returned paths",
                Some('S'),
            )
            .switch(
                "follow-symlinks",
                "Whether to follow symbolic links to their targets",
                Some('l'),
            )
            .named(
                "exclude",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Patterns to exclude from the search: `glob` will not walk the inside of directories matching the excluded patterns.",
                Some('e'),
            )
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Creates a list of files and/or folders based on the glob pattern provided."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "files", "folders", "list", "ls"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for *.rs files",
                example: "glob *.rs",
                result: None,
            },
            Example {
                description: "Search for *.rs and *.toml files recursively up to 2 folders deep",
                example: "glob **/*.{rs,toml} --depth 2",
                result: None,
            },
            Example {
                description: "Search for files and folders that begin with uppercase C or lowercase c",
                example: r#"glob "[Cc]*""#,
                result: None,
            },
            Example {
                description: "Search for files and folders like abc or xyz substituting a character for ?",
                example: r#"glob "{a?c,x?z}""#,
                result: None,
            },
            Example {
                description: "A case-insensitive search for files and folders that begin with c",
                example: r#"glob "(?i)c*""#,
                result: None,
            },
            Example {
                description: "Search for files or folders that do not begin with c, C, b, M, or s",
                example: r#"glob "[!cCbMs]*""#,
                result: None,
            },
            Example {
                description: "Search for files or folders with 3 a's in a row in the name",
                example: "glob <a*:3>",
                result: None,
            },
            Example {
                description: "Search for files or folders with only a, b, c, or d in the file name between 1 and 10 times",
                example: "glob <[a-d]:1,10>",
                result: None,
            },
            Example {
                description: "Search for folders that begin with an uppercase ASCII letter, ignoring files and symlinks",
                example: r#"glob "[A-Z]*" --no-file --no-symlink"#,
                result: None,
            },
            Example {
                description: "Search for files named tsconfig.json that are not in node_modules directories",
                example: r#"glob **/tsconfig.json --exclude [**/node_modules/**]"#,
                result: None,
            },
            Example {
                description: "Search for all files that are not in the target nor .git directories",
                example: r#"glob **/* --exclude [**/target/** **/.git/** */]"#,
                result: None,
            },
            Example {
                description: "Search for files following symbolic links to their targets",
                example: r#"glob "**/*.txt" --follow-symlinks"#,
                result: None,
            },
        ]
    }

    fn extra_description(&self) -> &str {
        r#"For more glob pattern help, please refer to https://docs.rs/crate/wax/latest"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let glob_pattern_input: Value = call.req(engine_state, stack, 0)?;
        let glob_span = glob_pattern_input.span();
        let depth = call.get_flag(engine_state, stack, "depth")?;
        let no_dirs = call.has_flag(engine_state, stack, "no-dir")?;
        let no_files = call.has_flag(engine_state, stack, "no-file")?;
        let no_symlinks = call.has_flag(engine_state, stack, "no-symlink")?;
        let follow_symlinks = call.has_flag(engine_state, stack, "follow-symlinks")?;
        let paths_to_exclude: Option<Value> = call.get_flag(engine_state, stack, "exclude")?;

        let (not_patterns, not_pattern_span): (Vec<String>, Span) = match paths_to_exclude {
            None => (vec![], span),
            Some(f) => {
                let pat_span = f.span();
                match f {
                    Value::List { vals: pats, .. } => {
                        let p = convert_patterns(pats.as_slice())?;
                        (p, pat_span)
                    }
                    _ => (vec![], span),
                }
            }
        };

        let glob_pattern =
            match glob_pattern_input {
                Value::String { val, .. } | Value::Glob { val, .. } => val,
                _ => return Err(ShellError::IncorrectValue {
                    msg: "Incorrect glob pattern supplied to glob. Please use string or glob only."
                        .to_string(),
                    val_span: call.head,
                    call_span: glob_span,
                }),
            };

        // paths starting with drive letters must be escaped on Windows
        #[cfg(windows)]
        let glob_pattern = patch_windows_glob_pattern(glob_pattern, glob_span)?;

        if glob_pattern.is_empty() {
            return Err(ShellError::GenericError {
                error: "glob pattern must not be empty".into(),
                msg: "glob pattern is empty".into(),
                span: Some(glob_span),
                help: Some("add characters to the glob pattern".into()),
                inner: vec![],
            });
        }

        // below we have to check / instead of MAIN_SEPARATOR because glob uses / as separator
        // using a glob like **\*.rs should fail because it's not a valid glob pattern
        let folder_depth = if let Some(depth) = depth {
            depth
        } else if glob_pattern.contains("**") {
            usize::MAX
        } else if glob_pattern.contains('/') {
            glob_pattern.split('/').count() + 1
        } else {
            1
        };

        let (prefix, glob) = match WaxGlob::new(&glob_pattern) {
            Ok(p) => p.partition(),
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "error with glob pattern".into(),
                    msg: format!("{e}"),
                    span: Some(glob_span),
                    help: None,
                    inner: vec![],
                });
            }
        };

        let path = engine_state.cwd_as_string(Some(stack))?;
        let path = match nu_path::canonicalize_with(prefix, path) {
            Ok(path) => path,
            Err(e) if e.to_string().contains("os error 2") =>
            // path we're trying to glob doesn't exist,
            {
                std::path::PathBuf::new() // user should get empty list not an error
            }
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "error in canonicalize".into(),
                    msg: format!("{e}"),
                    span: Some(glob_span),
                    help: None,
                    inner: vec![],
                });
            }
        };

        let link_behavior = match follow_symlinks {
            true => wax::LinkBehavior::ReadTarget,
            false => wax::LinkBehavior::ReadFile,
        };

        let result = if !not_patterns.is_empty() {
            let np: Vec<&str> = not_patterns.iter().map(|s| s as &str).collect();
            let glob_results = glob
                .walk_with_behavior(
                    path,
                    WalkBehavior {
                        depth: folder_depth,
                        link: link_behavior,
                    },
                )
                .into_owned()
                .not(np)
                .map_err(|err| ShellError::GenericError {
                    error: "error with glob's not pattern".into(),
                    msg: format!("{err}"),
                    span: Some(not_pattern_span),
                    help: None,
                    inner: vec![],
                })?
                .flatten();
            glob_to_value(
                engine_state.signals(),
                glob_results,
                no_dirs,
                no_files,
                no_symlinks,
                span,
            )
        } else {
            let glob_results = glob
                .walk_with_behavior(
                    path,
                    WalkBehavior {
                        depth: folder_depth,
                        link: link_behavior,
                    },
                )
                .into_owned()
                .flatten();
            glob_to_value(
                engine_state.signals(),
                glob_results,
                no_dirs,
                no_files,
                no_symlinks,
                span,
            )
        };

        Ok(result.into_pipeline_data(span, engine_state.signals().clone()))
    }
}

#[cfg(windows)]
fn patch_windows_glob_pattern(glob_pattern: String, glob_span: Span) -> Result<String, ShellError> {
    let mut chars = glob_pattern.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(drive), Some(':'), Some('/' | '\\')) if drive.is_ascii_alphabetic() => {
            Ok(format!("{drive}\\:/{}", chars.as_str()))
        }
        (Some(drive), Some(':'), Some(_)) if drive.is_ascii_alphabetic() => {
            Err(ShellError::GenericError {
                error: "invalid Windows path format".into(),
                msg: "Windows paths with drive letters must include a path separator (/) after the colon".into(),
                span: Some(glob_span),
                help: Some("use format like 'C:/' instead of 'C:'".into()),
                inner: vec![],
            })
        }
        _ => Ok(glob_pattern),
    }
}

fn convert_patterns(columns: &[Value]) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value {
            Value::String { val: s, .. } => Ok(s.clone()),
            _ => Err(ShellError::IncompatibleParametersSingle {
                msg: "Incorrect column format, Only string as column name".to_string(),
                span: value.span(),
            }),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

fn glob_to_value(
    signals: &Signals,
    glob_results: impl Iterator<Item = WalkEntry<'static>> + Send + 'static,
    no_dirs: bool,
    no_files: bool,
    no_symlinks: bool,
    span: Span,
) -> ListStream {
    let map_signals = signals.clone();
    let result = glob_results.filter_map(move |entry| {
        if let Err(err) = map_signals.check(&span) {
            return Some(Value::error(err, span));
        };
        let file_type = entry.file_type();

        if !(no_dirs && file_type.is_dir()
            || no_files && file_type.is_file()
            || no_symlinks && file_type.is_symlink())
        {
            Some(Value::string(
                entry.into_path().to_string_lossy().to_string(),
                span,
            ))
        } else {
            None
        }
    });

    ListStream::new(result, span, signals.clone())
}

#[cfg(windows)]
#[cfg(test)]
mod windows_tests {
    use super::*;

    #[test]
    fn glob_pattern_with_drive_letter() {
        let pattern = "D:/*.mp4".to_string();
        let result = patch_windows_glob_pattern(pattern, Span::test_data()).unwrap();
        assert!(WaxGlob::new(&result).is_ok());

        let pattern = "Z:/**/*.md".to_string();
        let result = patch_windows_glob_pattern(pattern, Span::test_data()).unwrap();
        assert!(WaxGlob::new(&result).is_ok());

        let pattern = "C:/nested/**/escaped/path/<[_a-zA-Z\\-]>.md".to_string();
        let result = patch_windows_glob_pattern(pattern, Span::test_data()).unwrap();
        assert!(dbg!(WaxGlob::new(&result)).is_ok());
    }

    #[test]
    fn glob_pattern_without_drive_letter() {
        let pattern = "/usr/bin/*.sh".to_string();
        let result = patch_windows_glob_pattern(pattern.clone(), Span::test_data()).unwrap();
        assert_eq!(result, pattern);
        assert!(WaxGlob::new(&result).is_ok());

        let pattern = "a".to_string();
        let result = patch_windows_glob_pattern(pattern.clone(), Span::test_data()).unwrap();
        assert_eq!(result, pattern);
        assert!(WaxGlob::new(&result).is_ok());
    }

    #[test]
    fn invalid_path_format() {
        let invalid = "C:lol".to_string();
        let result = patch_windows_glob_pattern(invalid, Span::test_data());
        assert!(result.is_err());
    }

    #[test]
    fn unpatched_patterns() {
        let unpatched = "C:/Users/*.txt".to_string();
        assert!(WaxGlob::new(&unpatched).is_err());

        let patched = patch_windows_glob_pattern(unpatched, Span::test_data()).unwrap();
        assert!(WaxGlob::new(&patched).is_ok());
    }
}
