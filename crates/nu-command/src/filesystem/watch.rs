use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use nu_engine::{current_dir, eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    format_error, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature,
    Spanned, SyntaxShape, Value,
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

    fn usage(&self) -> &str {
        "Watch for file changes and execute Nu code when they happen."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["watcher", "reload", "filesystem"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("watch")
            .required("path", SyntaxShape::Filepath, "the path to watch. Can be a file or directory")
            .required("block", SyntaxShape::Block, "A Nu block of code to run whenever a file changes. The block will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order")
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
            .switch("verbose", "Operate in verbose mode (default: false)", Some('v'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cwd = current_dir(engine_state, stack)?;
        let path_arg: Spanned<String> = call.req(engine_state, stack, 0)?;

        let path_no_whitespace = &path_arg
            .item
            .trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

        let path = match nu_path::canonicalize_with(path_no_whitespace, cwd) {
            Ok(p) => p,
            Err(e) => {
                return Err(ShellError::DirectoryNotFound(
                    path_arg.span,
                    Some(format!("IO Error: {:?}", e)),
                ))
            }
        };

        let capture_block: Closure = call.req(engine_state, stack, 1)?;
        let block = engine_state
            .clone()
            .get_block(capture_block.block_id)
            .clone();

        let verbose = call.has_flag("verbose");

        let debounce_duration_flag: Option<Spanned<i64>> =
            call.get_flag(engine_state, stack, "debounce-ms")?;
        let debounce_duration = match debounce_duration_flag {
            Some(val) => match u64::try_from(val.item) {
                Ok(val) => Duration::from_millis(val),
                Err(_) => {
                    return Err(ShellError::UnsupportedInput(
                        "Input out of range".to_string(),
                        val.span,
                    ))
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
                    Err(_) => return Err(ShellError::UnsupportedInput("".to_string(), glob.span)),
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

        let ctrlc_ref = &engine_state.ctrlc.clone();
        let (tx, rx) = channel();

        let mut watcher: RecommendedWatcher = match Watcher::new(tx, debounce_duration) {
            Ok(w) => w,
            Err(e) => {
                return Err(ShellError::IOError(format!(
                    "Failed to create watcher: {e}"
                )))
            }
        };

        if let Err(e) = watcher.watch(path.clone(), recursive_mode) {
            return Err(ShellError::IOError(format!("Failed to start watcher: {e}")));
        }

        eprintln!("Now watching files at {path:?}. Press ctrl+c to abort.");

        let event_handler =
            |operation: &str, path: PathBuf, new_path: Option<PathBuf>| -> Result<(), ShellError> {
                let glob_pattern = glob_pattern.clone();
                let matches_glob = match glob_pattern.clone() {
                    Some(glob) => glob.matches_path(&path),
                    None => true,
                };
                if verbose && glob_pattern.is_some() {
                    eprintln!("Matches glob: {matches_glob}");
                }

                if matches_glob {
                    let stack = &mut stack.clone();

                    if let Some(position) = block.signature.get_positional(0) {
                        if let Some(position_id) = &position.var_id {
                            stack.add_var(
                                *position_id,
                                Value::String {
                                    val: operation.to_string(),
                                    span: call.span(),
                                },
                            );
                        }
                    }

                    if let Some(position) = block.signature.get_positional(1) {
                        if let Some(position_id) = &position.var_id {
                            stack.add_var(
                                *position_id,
                                Value::String {
                                    val: path.to_string_lossy().to_string(),
                                    span: call.span(),
                                },
                            );
                        }
                    }

                    if let Some(position) = block.signature.get_positional(2) {
                        if let Some(position_id) = &position.var_id {
                            stack.add_var(
                                *position_id,
                                Value::String {
                                    val: new_path
                                        .unwrap_or_else(|| "".into())
                                        .to_string_lossy()
                                        .to_string(),
                                    span: call.span(),
                                },
                            );
                        }
                    }

                    let eval_result = eval_block(
                        engine_state,
                        stack,
                        &block,
                        Value::Nothing { span: call.span() }.into_pipeline_data(),
                        call.redirect_stdout,
                        call.redirect_stderr,
                    );

                    match eval_result {
                        Ok(val) => {
                            val.print(engine_state, stack, false, false)?;
                        }
                        Err(err) => {
                            let working_set = StateWorkingSet::new(engine_state);
                            eprintln!("{}", format_error(&working_set, &err));
                        }
                    }
                }

                Ok(())
            };

        loop {
            match rx.recv_timeout(CHECK_CTRL_C_FREQUENCY) {
                Ok(event) => {
                    if verbose {
                        eprintln!("{:?}", event);
                    }
                    let handler_result = match event {
                        DebouncedEvent::Create(path) => event_handler("Create", path, None),
                        DebouncedEvent::Write(path) => event_handler("Write", path, None),
                        DebouncedEvent::Remove(path) => event_handler("Remove", path, None),
                        DebouncedEvent::Rename(path, new_path) => {
                            event_handler("Rename", path, Some(new_path))
                        }
                        DebouncedEvent::Error(err, path) => match path {
                            Some(path) => Err(ShellError::IOError(format!(
                                "Error detected for {path:?}: {err:?}"
                            ))),
                            None => Err(ShellError::IOError(format!("Error detected: {err:?}"))),
                        },
                        // These are less likely to be interesting events
                        DebouncedEvent::Chmod(_)
                        | DebouncedEvent::NoticeRemove(_)
                        | DebouncedEvent::NoticeWrite(_)
                        | DebouncedEvent::Rescan => Ok(()),
                    };
                    handler_result?;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(ShellError::IOError(
                        "Unexpected disconnect from file watcher".into(),
                    ));
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            if let Some(ctrlc) = ctrlc_ref {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run `cargo test` whenever a Rust file changes",
                example: r#"watch . --glob=**/*.rs { cargo test }"#,
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
        ]
    }
}
