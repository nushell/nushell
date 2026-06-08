use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals, shell_error::generic::GenericError};
use wax::{
    Glob as WaxGlob, any, walk::DepthBehavior, walk::DepthMax, walk::Entry, walk::FileIterator,
    walk::GlobEntry, walk::LinkBehavior, walk::WalkBehavior,
};

#[derive(Clone)]
pub struct Glob;

impl Command for Glob {
    fn name(&self) -> &str {
        "glob"
    }

    fn signature(&self) -> Signature {
        let signature = Signature::build("glob")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .required(
                "glob",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::GlobPattern]),
                "The glob expression.",
            )
            .named(
                "depth",
                SyntaxShape::Int,
                "Directory depth to search.",
                Some('d'),
            )
            .switch(
                "no-dir",
                "Whether to filter out directories from the returned paths.",
                Some('D'),
            )
            .switch(
                "no-file",
                "Whether to filter out files from the returned paths.",
                Some('F'),
            )
            .switch(
                "no-symlink",
                "Whether to filter out symlinks from the returned paths.",
                Some('S'),
            )
            .switch(
                "follow-symlinks",
                "Whether to follow symbolic links to their targets.",
                Some('l'),
            )
            .named(
                "exclude",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Patterns to exclude from the search: `glob` will not walk the inside of directories matching the excluded patterns.",
                Some('e'),
            )
            .category(Category::FileSystem);

        if !nu_experimental::DC_GLOB.get() {
            return signature;
        }

        signature
            .rest(
                "debug-args",
                SyntaxShape::String,
                "Additional positional args used by --dbg-matches/--dbg-glob.",
            )
            .switch(
                "dbg-parse",
                "Use dc-glob debug parser mode. Requires one positional pattern.",
                None,
            )
            .switch(
                "dbg-compile",
                "Use dc-glob debug compile mode. Requires one positional pattern.",
                None,
            )
            .switch(
                "dbg-matches",
                "Use dc-glob debug match mode. Requires pattern and optional path positional args.",
                None,
            )
            .switch(
                "dbg-glob",
                "Use dc-glob debug glob mode. Requires pattern and optional relative-to positional args.",
                None,
            )
    }

    fn description(&self) -> &str {
        "Creates a list of files and/or folders based on the glob pattern provided."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "files", "folders", "list", "ls"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Search for *.rs files.",
                example: "glob *.rs",
                result: None,
            },
            Example {
                description: "Search for *.rs and *.toml files recursively up to 2 folders deep.",
                example: "glob **/*.{rs,toml} --depth 2",
                result: None,
            },
            Example {
                description: "Search for files and folders that begin with uppercase C or lowercase c.",
                example: r#"glob "[Cc]*""#,
                result: None,
            },
            Example {
                description: "Search for files and folders like abc or xyz substituting a character for ?.",
                example: r#"glob "{a?c,x?z}""#,
                result: None,
            },
            Example {
                description: "A case-insensitive search for files and folders that begin with c.",
                example: r#"glob "(?i)c*""#,
                result: None,
            },
            Example {
                description: "Search for files or folders that do not begin with c, C, b, M, or s.",
                example: r#"glob "[!cCbMs]*""#,
                result: None,
            },
            Example {
                description: "Search for files or folders with 3 a's in a row in the name.",
                example: "glob <a*:3>",
                result: None,
            },
            Example {
                description: "Search for files or folders with only a, b, c, or d in the file name between 1 and 10 times.",
                example: "glob <[a-d]:1,10>",
                result: None,
            },
            Example {
                description: "Search for folders that begin with an uppercase ASCII letter, ignoring files and symlinks.",
                example: r#"glob "[A-Z]*" --no-file --no-symlink"#,
                result: None,
            },
            Example {
                description: "Search for files named tsconfig.json that are not in node_modules directories.",
                example: "glob **/tsconfig.json --exclude [**/node_modules/**]",
                result: None,
            },
            Example {
                description: "Search for all files that are not in the target nor .git directories.",
                example: "glob **/* --exclude [**/target/** **/.git/** */]",
                result: None,
            },
            Example {
                description: "Search for files following symbolic links to their targets.",
                example: r#"glob "**/*.txt" --follow-symlinks"#,
                result: None,
            },
        ]
    }

    fn extra_description(&self) -> &str {
        if nu_experimental::DC_GLOB.get() {
            ""
        } else {
            "For more glob pattern help, please refer to https://docs.rs/crate/wax/latest."
        }
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

        if nu_experimental::DC_GLOB.get() {
            let has_dbg_flags = call.get_flag_span(stack, "dbg-parse").is_some()
                || call.get_flag_span(stack, "dbg-compile").is_some()
                || call.get_flag_span(stack, "dbg-matches").is_some()
                || call.get_flag_span(stack, "dbg-glob").is_some();

            if has_dbg_flags {
                let dbg_parse = call.has_flag(engine_state, stack, "dbg-parse")?;
                let dbg_compile = call.has_flag(engine_state, stack, "dbg-compile")?;
                let dbg_matches = call.has_flag(engine_state, stack, "dbg-matches")?;
                let dbg_glob = call.has_flag(engine_state, stack, "dbg-glob")?;

                let dbg_modes = [dbg_parse, dbg_compile, dbg_matches, dbg_glob]
                    .into_iter()
                    .filter(|set| *set)
                    .count();

                if dbg_modes > 1 {
                    return Err(ShellError::IncompatibleParametersSingle {
                        msg:
                            "use only one of --dbg-parse, --dbg-compile, --dbg-matches, --dbg-glob"
                                .to_string(),
                        span,
                    });
                }

                if dbg_modes == 1 {
                    let args = call.rest::<Spanned<String>>(engine_state, stack, 0)?;
                    let subcommand = if dbg_parse {
                        "dbg-parse"
                    } else if dbg_compile {
                        "dbg-compile"
                    } else if dbg_matches {
                        "dbg-matches"
                    } else {
                        "dbg-glob"
                    };

                    return run_debug_subcommand(
                        engine_state,
                        stack,
                        subcommand.to_string(),
                        args,
                        span,
                    );
                }
            }
        }

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

        let extra_args = call.rest::<Spanned<String>>(engine_state, stack, 1)?;
        if !extra_args.is_empty() {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "extra positional argument".to_string(),
                span: extra_args[0].span,
            });
        }

        if glob_pattern.is_empty() {
            return Err(ShellError::Generic(
                GenericError::new(
                    "glob pattern must not be empty",
                    "glob pattern is empty",
                    glob_span,
                )
                .with_help("add characters to the glob pattern"),
            ));
        }

        match nu_experimental::DC_GLOB.get() {
            true => run_dc_glob(
                engine_state,
                stack,
                &glob_pattern,
                depth,
                follow_symlinks,
                not_patterns,
                glob_span,
                no_dirs,
                no_files,
                no_symlinks,
                span,
            ),
            false => {
                // paths starting with drive letters must be escaped for wax on Windows
                #[cfg(windows)]
                let glob_pattern = patch_windows_glob_pattern(glob_pattern, glob_span)?;

                run_legacy_glob(
                    engine_state,
                    stack,
                    &glob_pattern,
                    depth,
                    follow_symlinks,
                    not_patterns,
                    glob_span,
                    not_pattern_span,
                    no_dirs,
                    no_files,
                    no_symlinks,
                    span,
                )
            }
        }
    }
}

fn infer_folder_depth(glob_pattern: &str, depth: Option<usize>) -> usize {
    if let Some(depth) = depth {
        depth
    } else if glob_pattern.contains("**") {
        usize::MAX
    } else if glob_pattern.contains('/') {
        glob_pattern.split('/').count() + 1
    } else {
        1
    }
}

#[allow(clippy::too_many_arguments)]
fn run_dc_glob(
    engine_state: &EngineState,
    stack: &Stack,
    glob_pattern: &str,
    depth: Option<usize>,
    follow_symlinks: bool,
    not_patterns: Vec<String>,
    glob_span: Span,
    no_dirs: bool,
    no_files: bool,
    no_symlinks: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let folder_depth = infer_folder_depth(glob_pattern, depth);
    let cwd = engine_state.cwd(Some(stack))?;
    let options = nu_glob::dc_glob::GlobWalkOptions {
        max_depth: (folder_depth != usize::MAX).then_some(folder_depth),
        follow_symlinks,
        excludes: not_patterns,
        interrupt: engine_state.signals().interrupt_flag(),
    };
    let cwd_for_matches = cwd.as_std_path().to_path_buf();

    let matches =
        nu_glob::dc_glob::glob_with(cwd.as_std_path(), glob_pattern, &options).map_err(|err| {
            ShellError::Generic(GenericError::new(
                "error with glob pattern",
                err.to_string(),
                glob_span,
            ))
        })?;

    let matches = matches.map(move |item| {
        item.map(|path| {
            if path.is_absolute() {
                path
            } else {
                cwd_for_matches.join(path)
            }
        })
        .map_err(|err| {
            ShellError::Generic(GenericError::new(
                "error with glob pattern",
                err.to_string(),
                glob_span,
            ))
        })
    });

    let values = glob_paths_to_value(
        engine_state.signals(),
        matches,
        no_dirs,
        no_files,
        no_symlinks,
        span,
    );

    Ok(values.into_pipeline_data(span, engine_state.signals().clone()))
}

#[allow(clippy::too_many_arguments)]
fn run_legacy_glob(
    engine_state: &EngineState,
    stack: &Stack,
    glob_pattern: &str,
    depth: Option<usize>,
    follow_symlinks: bool,
    not_patterns: Vec<String>,
    glob_span: Span,
    not_pattern_span: Span,
    no_dirs: bool,
    no_files: bool,
    no_symlinks: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    // below we have to check / instead of MAIN_SEPARATOR because glob uses / as separator
    // using a glob like **\\*.rs should fail because it's not a valid glob pattern
    let folder_depth = infer_folder_depth(glob_pattern, depth);

    let (prefix, glob) = match WaxGlob::new(glob_pattern) {
        Ok(p) => p.partition_or_empty(),
        Err(e) => {
            return Err(ShellError::Generic(GenericError::new(
                "error with glob pattern",
                format!("{e}"),
                glob_span,
            )));
        }
    };

    let path = engine_state.cwd_as_string(Some(stack))?;
    let path = nu_path::absolute_with(prefix, path).map_err(|e| {
        ShellError::Generic(GenericError::new("invalid path", format!("{e}"), glob_span))
    })?;
    let path = match path.try_exists() {
        Ok(true) => path,
        Ok(false) => std::path::PathBuf::new(), // user should get empty list not an error
        Err(e) => {
            return Err(ShellError::Generic(GenericError::new(
                "error accessing path",
                format!("{e}"),
                glob_span,
            )));
        }
    };

    let link_behavior = match follow_symlinks {
        true => LinkBehavior::ReadTarget,
        false => LinkBehavior::ReadFile,
    };

    let make_walk_behavior = |depth: usize| WalkBehavior {
        depth: DepthBehavior::Max(DepthMax(depth)),
        link: link_behavior,
    };

    let result = if !not_patterns.is_empty() {
        let patterns: Vec<WaxGlob<'static>> = not_patterns
            .into_iter()
            .map(|pattern| {
                WaxGlob::new(&pattern)
                    .map_err(|err| {
                        ShellError::Generic(GenericError::new(
                            "error with glob's not pattern",
                            format!("{err}"),
                            not_pattern_span,
                        ))
                    })
                    .map(|g| g.into_owned())
            })
            .collect::<Result<_, _>>()?;

        let any_pattern = any(patterns).map_err(|err| {
            ShellError::Generic(GenericError::new(
                "error with glob's not pattern",
                format!("{err}"),
                not_pattern_span,
            ))
        })?;

        let glob_results = glob
            .walk_with_behavior(path, make_walk_behavior(folder_depth))
            .not(any_pattern)
            .map_err(|err| {
                ShellError::Generic(GenericError::new(
                    "error with glob's not pattern",
                    format!("{err}"),
                    not_pattern_span,
                ))
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
            .walk_with_behavior(path, make_walk_behavior(folder_depth))
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

#[cfg(windows)]
fn patch_windows_glob_pattern(glob_pattern: String, glob_span: Span) -> Result<String, ShellError> {
    let mut chars = glob_pattern.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(drive), Some(':'), Some('/' | '\\')) if drive.is_ascii_alphabetic() => {
            Ok(format!("{drive}\\:/{}", chars.as_str()))
        }
        (Some(drive), Some(':'), Some(_)) if drive.is_ascii_alphabetic() => {
            Err(ShellError::Generic(
                GenericError::new(
                    "invalid Windows path format",
                    "Windows paths with drive letters must include a path separator (/) after the colon",
                    glob_span,
                )
                .with_help("use format like 'C:/' instead of 'C:'"),
            ))
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
    glob_results: impl Iterator<Item = GlobEntry> + Send + 'static,
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
                entry.into_path().to_string_lossy().into_owned(),
                span,
            ))
        } else {
            None
        }
    });

    ListStream::new(result, span, signals.clone())
}

fn glob_paths_to_value(
    signals: &Signals,
    glob_results: impl Iterator<Item = Result<std::path::PathBuf, ShellError>> + Send + 'static,
    no_dirs: bool,
    no_files: bool,
    no_symlinks: bool,
    span: Span,
) -> ListStream {
    let map_signals = signals.clone();
    let needs_file_type = no_dirs || no_files || no_symlinks;
    let result = glob_results.filter_map(move |entry| {
        if let Err(err) = map_signals.check(&span) {
            return Some(Value::error(err, span));
        }

        let path = match entry {
            Ok(path) => path,
            Err(err) => return Some(Value::error(err, span)),
        };

        if !needs_file_type {
            return Some(Value::string(path.to_string_lossy().into_owned(), span));
        }

        let file_type = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta.file_type(),
            Err(_) => {
                return Some(Value::string(path.to_string_lossy().into_owned(), span));
            }
        };

        if !(no_dirs && file_type.is_dir()
            || no_files && file_type.is_file()
            || no_symlinks && file_type.is_symlink())
        {
            Some(Value::string(path.to_string_lossy().into_owned(), span))
        } else {
            None
        }
    });

    ListStream::new(result, span, signals.clone())
}

fn run_debug_subcommand(
    engine_state: &EngineState,
    stack: &Stack,
    subcommand: String,
    args: Vec<Spanned<String>>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let expected = match subcommand.as_str() {
        "dbg-parse" | "dbg-compile" => 1,
        "dbg-matches" | "dbg-glob" => 2,
        _ => 0,
    };

    if expected > 0 && args.len() > expected {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "extra positional argument".to_string(),
            span: args[expected].span,
        });
    }

    match (subcommand.as_str(), args.first()) {
        ("dbg-parse", Some(pattern)) => {
            let text = nu_glob::dc_glob::debug_parse(&pattern.item);

            Ok(Value::string(text, span).into_pipeline_data())
        }
        ("dbg-compile", Some(pattern)) => {
            let text = nu_glob::dc_glob::debug_compile(&pattern.item).map_err(|err| {
                ShellError::Generic(GenericError::new(
                    "failed to compile debug glob pattern",
                    err.to_string(),
                    pattern.span,
                ))
            })?;
            Ok(Value::string(text, span).into_pipeline_data())
        }
        ("dbg-matches", Some(pattern)) => {
            let path = args.get(1).map(|p| p.item.as_str()).unwrap_or(".");
            let matches = nu_glob::dc_glob::debug_matches(&pattern.item, path).map_err(|err| {
                ShellError::Generic(GenericError::new(
                    "failed to run debug match",
                    err.to_string(),
                    pattern.span,
                ))
            })?;
            Ok(Value::bool(matches, span).into_pipeline_data())
        }
        ("dbg-glob", Some(pattern)) => {
            let pattern_span = pattern.span;
            let relative_to = args.get(1).map(|p| p.item.as_str()).unwrap_or(".");
            let cwd = engine_state.cwd(Some(stack))?;
            let relative_to =
                nu_path::absolute_with(relative_to, cwd.as_std_path()).map_err(|err| {
                    ShellError::Generic(GenericError::new(
                        "invalid debug glob path",
                        err.to_string(),
                        span,
                    ))
                })?;
            let out = nu_glob::dc_glob::glob_with(
                relative_to,
                &pattern.item,
                &nu_glob::dc_glob::GlobWalkOptions::default(),
            )
            .map_err(|err| {
                ShellError::Generic(GenericError::new(
                    "failed to run debug glob",
                    err.to_string(),
                    pattern.span,
                ))
            })?;

            let values = out.map(move |path| match path {
                Ok(path) => Value::string(path.to_string_lossy().into_owned(), span),
                Err(err) => Value::error(
                    ShellError::Generic(GenericError::new(
                        "failed to run debug glob",
                        err.to_string(),
                        pattern_span,
                    )),
                    span,
                ),
            });
            Ok(
                ListStream::new(values, span, engine_state.signals().clone())
                    .into_pipeline_data(span, engine_state.signals().clone()),
            )
        }
        ("dbg-parse" | "dbg-compile" | "dbg-matches" | "dbg-glob", None) => {
            Err(ShellError::MissingParameter {
                param_name: "pattern".to_string(),
                span,
            })
        }
        (unknown, _) => Err(ShellError::IncompatibleParametersSingle {
            msg: format!("unknown debug subcommand '{unknown}'"),
            span,
        }),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_mentions_dbg_subcommands() {
        let signature = Glob.signature();
        let rendered = format!("{signature:#?}");

        if nu_experimental::DC_GLOB.get() {
            assert!(
                rendered.contains("dbg-parse") && rendered.contains("dbg-glob"),
                "glob signature should mention dbg-* subcommands when dc-glob is enabled"
            );
        } else {
            assert!(
                !rendered.contains("dbg-parse") && !rendered.contains("dbg-glob"),
                "glob signature should hide dbg-* subcommands when dc-glob is disabled"
            );
        }
    }
}
