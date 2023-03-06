use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use nu_engine::{current_dir, eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    format_error, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature,
    Spanned, SyntaxShape, Type, Value,
};

// durations chosen mostly arbitrarily
const CHECK_CTRL_C_FREQUENCY: Duration = Duration::from_millis(100);

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
        .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .required("path", SyntaxShape::Filepath, "the path to watch. Can be a file or directory")
            .required("closure",
            SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::String, SyntaxShape::String])),
                "Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order")
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
    ) -> Result<PipelineData, ShellError> {
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
                    Some(format!("IO Error: {e:?}")),
                ))
            }
        };

        let capture_block: Closure = call.req(engine_state, stack, 1)?;
        let block = engine_state
            .clone()
            .get_block(capture_block.block_id)
            .clone();

        let verbose = call.has_flag("verbose");
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
                        })
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

        let ctrlc_ref = &engine_state.ctrlc.clone();
        let (tx, rx) = channel();

        let mut watcher = match recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                return Err(ShellError::IOError(format!(
                    "Failed to create watcher: {e}"
                )))
            }
        };
        if let Err(e) = watcher.watch(&path, recursive_mode) {
            return Err(ShellError::IOError(format!("Failed to start watcher: {e}")));
        }

        eprintln!("Now watching files at {path:?}. Press ctrl+c to abort.");

        let event_handler = |operation: &str, path: PathBuf| -> Result<(), ShellError> {
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
                        stack.add_var(*position_id, Value::string(operation, call.span()));
                    }
                }

                if let Some(position) = block.signature.get_positional(1) {
                    if let Some(position_id) = &position.var_id {
                        stack.add_var(
                            *position_id,
                            Value::string(path.to_string_lossy(), call.span()),
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
                Ok(Ok(mut event)) => {
                    if verbose {
                        eprintln!("{event:?}");
                    }
                    let path = match event.paths.pop() {
                        None => continue,
                        Some(p) => p,
                    };
                    match event.kind {
                        EventKind::Create(_) => event_handler("Create", path),
                        EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                            event_handler("Write", path)
                        }
                        EventKind::Remove(_) => event_handler("Remove", path),
                        _ => Ok(()),
                    }?
                }
                Ok(Err(e)) => return Err(ShellError::IOError(format!("watch error: {e}"))),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(ShellError::IOError(
                        "Unexpected disconnect from file watcher".into(),
                    ));
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            if nu_utils::ctrl_c::was_pressed(ctrlc_ref) {
                break;
            }
        }

        Ok(PipelineData::empty())
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
                example: r#"watch . { |op, path| $"($op) ($path)"}"#,
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
