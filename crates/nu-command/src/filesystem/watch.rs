use notify_debouncer_full::{
    new_debouncer,
    notify::{
        EventKind, RecursiveMode, Watcher,
        event::{DataChange, ModifyKind, RenameMode},
    },
};
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{engine::Closure, report_shell_error, shell_error::io::IoError};
use std::{
    path::PathBuf,
    sync::mpsc::{RecvTimeoutError, channel},
    time::Duration,
};

// durations chosen mostly arbitrarily
const CHECK_CTRL_C_FREQUENCY: Duration = Duration::from_millis(100);
const DEFAULT_WATCH_DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct Watch;

impl Command for Watch {
    fn name(&self) -> &str {
        "watch"
    }

    fn description(&self) -> &str {
        "Watch for file changes and execute Nu code when they happen."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["watcher", "reload", "filesystem"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("watch")
        .input_output_types(vec![(Type::Nothing, Type::table())])
            .required("path", SyntaxShape::Filepath, "The path to watch. Can be a file or directory.")
            .required("closure",
            SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::String, SyntaxShape::String])),
                "Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order.")
            .named(
                "debounce-ms",
                SyntaxShape::Int,
                "Debounce changes for this many milliseconds (default: 100). Adjust if you find that single writes are reported as multiple events",
                Some('d'),
            )
            .named(
                "glob",
                SyntaxShape::String, // SyntaxShape::GlobPattern gets interpreted relative to cwd, so use String instead
                "Only report changes for files that match this glob pattern (default: all files)",
                Some('g'),
            )
            .named(
                "recursive",
                SyntaxShape::Boolean,
                "Watch all directories under `<path>` recursively. Will be ignored if `<path>` is a file (default: true)",
                Some('r'),
            )
            .switch("quiet", "Hide the initial status message (default: false)", Some('q'))
            .switch("verbose", "Operate in verbose mode (default: false)", Some('v'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let cwd = engine_state.cwd_as_string(Some(stack))?;
        let path_arg: Spanned<String> = call.req(engine_state, stack, 0)?;

        let path_no_whitespace = &path_arg
            .item
            .trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

        let path = match nu_path::canonicalize_with(path_no_whitespace, cwd) {
            Ok(p) => p,
            Err(err) => {
                return Err(ShellError::Io(IoError::new(
                    err,
                    path_arg.span,
                    PathBuf::from(path_no_whitespace),
                )));
            }
        };

        let closure: Closure = call.req(engine_state, stack, 1)?;

        let verbose = call.has_flag(engine_state, stack, "verbose")?;

        let quiet = call.has_flag(engine_state, stack, "quiet")?;

        let debounce_duration_flag: Option<Spanned<i64>> =
            call.get_flag(engine_state, stack, "debounce-ms")?;
        let debounce_duration = match debounce_duration_flag {
            Some(val) => match u64::try_from(val.item) {
                Ok(val) => Duration::from_millis(val),
                Err(_) => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Debounce duration is invalid".to_string(),
                        span: val.span,
                    });
                }
            },
            None => DEFAULT_WATCH_DEBOUNCE_DURATION,
        };

        let glob_flag: Option<Spanned<String>> = call.get_flag(engine_state, stack, "glob")?;
        let glob_pattern = match glob_flag {
            Some(glob) => {
                let absolute_path = path.join(glob.item);
                if verbose {
                    eprintln!("Absolute glob path: {absolute_path:?}");
                }

                match nu_glob::Pattern::new(&absolute_path.to_string_lossy()) {
                    Ok(pattern) => Some(pattern),
                    Err(_) => {
                        return Err(ShellError::TypeMismatch {
                            err_message: "Glob pattern is invalid".to_string(),
                            span: glob.span,
                        });
                    }
                }
            }
            None => None,
        };

        let recursive_flag: Option<Spanned<bool>> =
            call.get_flag(engine_state, stack, "recursive")?;
        let recursive_mode = match recursive_flag {
            Some(recursive) => {
                if recursive.item {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                }
            }
            None => RecursiveMode::Recursive,
        };

        let (tx, rx) = channel();

        let mut debouncer = match new_debouncer(debounce_duration, None, tx) {
            Ok(d) => d,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "Failed to create watcher".to_string(),
                    msg: e.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                });
            }
        };
        if let Err(e) = debouncer.watcher().watch(&path, recursive_mode) {
            return Err(ShellError::GenericError {
                error: "Failed to create watcher".to_string(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }
        // need to cache to make sure that rename event works.
        debouncer.cache().add_root(&path, recursive_mode);

        if !quiet {
            eprintln!("Now watching files at {path:?}. Press ctrl+c to abort.");
        }

        let mut closure = ClosureEval::new(engine_state, stack, closure);

        let mut event_handler = move |operation: &str,
                                      path: PathBuf,
                                      new_path: Option<PathBuf>|
              -> Result<(), ShellError> {
            let matches_glob = match &glob_pattern {
                Some(glob) => glob.matches_path(&path),
                None => true,
            };
            if verbose && glob_pattern.is_some() {
                eprintln!("Matches glob: {matches_glob}");
            }

            if matches_glob {
                let result = closure
                    .add_arg(Value::string(operation, head))
                    .add_arg(Value::string(path.to_string_lossy(), head))
                    .add_arg(Value::string(
                        new_path.unwrap_or_else(|| "".into()).to_string_lossy(),
                        head,
                    ))
                    .run_with_input(PipelineData::Empty);

                match result {
                    Ok(val) => val.print_table(engine_state, stack, false, false)?,
                    Err(err) => report_shell_error(engine_state, &err),
                };
            }

            Ok(())
        };

        loop {
            match rx.recv_timeout(CHECK_CTRL_C_FREQUENCY) {
                Ok(Ok(events)) => {
                    if verbose {
                        eprintln!("{events:?}");
                    }
                    for mut one_event in events {
                        let handle_result = match one_event.event.kind {
                            // only want to handle event if relative path exists.
                            EventKind::Create(_) => one_event
                                .paths
                                .pop()
                                .map(|path| event_handler("Create", path, None))
                                .unwrap_or(Ok(())),
                            EventKind::Remove(_) => one_event
                                .paths
                                .pop()
                                .map(|path| event_handler("Remove", path, None))
                                .unwrap_or(Ok(())),
                            EventKind::Modify(ModifyKind::Data(DataChange::Content))
                            | EventKind::Modify(ModifyKind::Data(DataChange::Any))
                            | EventKind::Modify(ModifyKind::Any) => one_event
                                .paths
                                .pop()
                                .map(|path| event_handler("Write", path, None))
                                .unwrap_or(Ok(())),
                            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => one_event
                                .paths
                                .pop()
                                .map(|to| {
                                    one_event
                                        .paths
                                        .pop()
                                        .map(|from| event_handler("Rename", from, Some(to)))
                                        .unwrap_or(Ok(()))
                                })
                                .unwrap_or(Ok(())),
                            _ => Ok(()),
                        };
                        handle_result?;
                    }
                }
                Ok(Err(_)) => {
                    return Err(ShellError::GenericError {
                        error: "Receiving events failed".to_string(),
                        msg: "Unexpected errors when receiving events".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    });
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(ShellError::GenericError {
                        error: "Disconnected".to_string(),
                        msg: "Unexpected disconnect from file watcher".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    });
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            if engine_state.signals().interrupted() {
                break;
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run `cargo test` whenever a Rust file changes",
                example: r#"watch . --glob=**/*.rs {|| cargo test }"#,
                result: None,
            },
            Example {
                description: "Watch all changes in the current directory",
                example: r#"watch . { |op, path, new_path| $"($op) ($path) ($new_path)"}"#,
                result: None,
            },
            Example {
                description: "Log all changes in a directory",
                example: r#"watch /foo/bar { |op, path| $"($op) - ($path)(char nl)" | save --append changes_in_bar.log }"#,
                result: None,
            },
            Example {
                description: "Note: if you are looking to run a command every N units of time, this can be accomplished with a loop and sleep",
                example: r#"loop { command; sleep duration }"#,
                result: None,
            },
        ]
    }
}
