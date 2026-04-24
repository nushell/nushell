use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
    time::Duration,
};

use itertools::Either;
use notify_debouncer_full::{
    DebouncedEvent, Debouncer, FileIdMap, new_debouncer,
    notify::{
        self, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
        event::{DataChange, ModifyKind, RenameMode},
    },
};

use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{
    Signals, engine::Closure, report_shell_error, shell_error::generic::GenericError,
    shell_error::io::IoError,
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

    fn extra_description(&self) -> &str {
        "When run without a closure, `watch` returns a stream of events instead."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["watcher", "reload", "filesystem"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("watch")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (
                    Type::Nothing,
                    Type::Table(vec![
                        ("operation".into(), Type::String),
                        ("path".into(), Type::OneOf([Type::String, Type::Nothing].into())),
                        ("new_path".into(), Type::OneOf([Type::String, Type::Nothing].into())),
                    ].into_boxed_slice())
                ),
            ])
            .required("path", SyntaxShape::Filepath, "The path to watch. Can be a file or directory.")
            .optional(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::String, SyntaxShape::String])),
                "Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order.",
            )
            .named(
                "debounce",
                SyntaxShape::Duration,
                "Debounce changes for this duration (default: 100ms). Adjust if you find that single writes are reported as multiple events.",
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
                "Watch all directories under `<path>` recursively. Will be ignored if `<path>` is a file (default: true).",
                Some('r'),
            )
            .switch("quiet", "Hide the initial status message (default: false).", Some('q'))
            .switch("verbose", "Operate in verbose mode (default: false).", Some('v'))
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

        let path = {
            let cwd = engine_state.cwd_as_string(Some(stack))?;
            let path_arg: Spanned<String> = call.req(engine_state, stack, 0)?;
            let path_no_whitespace = path_arg
                .item
                .trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

            nu_path::absolute_with(path_no_whitespace, cwd).map_err(|err| {
                ShellError::Io(IoError::new(
                    err,
                    path_arg.span,
                    PathBuf::from(path_no_whitespace),
                ))
            })?
        };
        let closure: Option<Closure> = call.opt(engine_state, stack, 1)?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let quiet = call.has_flag(engine_state, stack, "quiet")?;
        let debounce_duration: Duration = call
            .get_flag(engine_state, stack, "debounce")?
            .unwrap_or(DEFAULT_WATCH_DEBOUNCE_DURATION);

        let glob_flag: Option<Spanned<String>> = call.get_flag(engine_state, stack, "glob")?;
        let glob_pattern = glob_flag
            .map(|glob| {
                let absolute_path = path.join(glob.item);
                if verbose {
                    eprintln!("Absolute glob path: {absolute_path:?}");
                }

                nu_glob::Pattern::new(&absolute_path.to_string_lossy()).map_err(|_| {
                    ShellError::TypeMismatch {
                        err_message: "Glob pattern is invalid".to_string(),
                        span: glob.span,
                    }
                })
            })
            .transpose()?;

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

        let mut debouncer = new_debouncer(debounce_duration, None, tx).map_err(|err| {
            ShellError::Generic(GenericError::new(
                "Failed to create watcher",
                err.to_string(),
                call.head,
            ))
        })?;

        if let Err(err) = debouncer.watcher().watch(&path, recursive_mode) {
            return Err(ShellError::Generic(GenericError::new(
                "Failed to create watcher",
                err.to_string(),
                call.head,
            )));
        }
        // need to cache to make sure that rename event works.
        debouncer.cache().add_root(&path, recursive_mode);

        if !quiet {
            eprintln!("Now watching files at {path:?}. Press ctrl+c to abort.");
        }

        let iter = WatchIterator::new(debouncer, rx, engine_state.signals().clone());

        fn glob_filter(glob: Option<&nu_glob::Pattern>, ev: &WatchEvent) -> bool {
            let Some(glob) = glob else { return true };
            let path = ev
                .path
                .as_deref()
                .or(ev.new_path.as_deref())
                .expect("at least one of path or new_path should be present");
            glob.matches_path(path)
        }

        if let Some(closure) = closure {
            let mut closure = ClosureEval::new(engine_state, stack, closure);

            for events in iter {
                for event in events? {
                    let matches_glob = glob_filter(glob_pattern.as_ref(), &event);

                    if verbose && glob_pattern.is_some() {
                        eprintln!("Matches glob: {matches_glob}");
                    }

                    if matches_glob {
                        let result = closure
                            .add_arg(event.operation.into_value(head))?
                            .add_arg(event.path.into_value(head))?
                            .add_arg(event.new_path.into_value(head))?
                            .run_with_input(PipelineData::empty());

                        match result {
                            Ok(val) => val.print_table(engine_state, stack, false, false)?,
                            Err(err) => report_shell_error(Some(stack), engine_state, &err),
                        };
                    }
                }
            }

            Ok(PipelineData::empty())
        } else {
            let out = iter
                .flat_map(|e| match e {
                    Ok(events) => Either::Right(events.into_iter().map(Ok)),
                    Err(err) => Either::Left(std::iter::once(Err(err))),
                })
                .filter_map(move |e| match e {
                    Ok(ev) if glob_filter(glob_pattern.as_ref(), &ev) => Some(ev.into_value(head)),
                    Ok(_) => None,
                    Err(err) => Some(Value::error(err, head)),
                })
                .into_pipeline_data(head, engine_state.signals().clone());
            Ok(out)
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Run `cargo test` whenever a Rust file changes.",
                example: "watch . --glob=**/*.rs {|| cargo test }",
                result: None,
            },
            Example {
                description: "Watch all changes in the current directory.",
                example: r#"watch . { |op, path, new_path| $"($op) ($path) ($new_path)"}"#,
                result: None,
            },
            Example {
                description: "`watch` (when run without a closure) can also emit a stream of events it detects.",
                example: r#"watch /foo/bar
    | where operation == Create
    | first 5
    | each {|e| $"New file!: ($e.path)" }
    | to text
    | save --append changes_in_bar.log"#,
                result: None,
            },
            Example {
                description: "Print file changes with a debounce time of 5 minutes.",
                example: r#"watch /foo/bar --debounce 5min { |op, path| $"Registered ($op) on ($path)" | print }"#,
                result: None,
            },
            Example {
                description: "Note: if you are looking to run a command every N units of time, this can be accomplished with a loop and sleep.",
                example: "loop { command; sleep duration }",
                result: None,
            },
        ]
    }
}

#[derive(IntoValue)]
struct WatchEvent {
    operation: WatchEventKind,
    path: Option<PathBuf>,
    new_path: Option<PathBuf>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "UpperCamelCase")]
enum WatchEventKind {
    Create,
    Write,
    Rename,
    Remove,
}

impl TryFrom<EventKind> for WatchEventKind {
    type Error = ();

    fn try_from(value: EventKind) -> Result<Self, Self::Error> {
        Ok(match value {
            EventKind::Create(_) => Self::Create,
            EventKind::Remove(_) => Self::Remove,
            EventKind::Modify(
                ModifyKind::Data(DataChange::Content | DataChange::Any) | ModifyKind::Any,
            ) => Self::Write,
            EventKind::Modify(ModifyKind::Name(
                RenameMode::Both | RenameMode::From | RenameMode::To,
            )) => Self::Rename,
            _ => return Err(()),
        })
    }
}

impl TryFrom<DebouncedEvent> for WatchEvent {
    type Error = ();

    fn try_from(ev: DebouncedEvent) -> Result<Self, Self::Error> {
        // TODO: Maybe we should handle all event kinds?
        let DebouncedEvent {
            event: notify::Event {
                kind, mut paths, ..
            },
            ..
        } = ev;

        let (path, new_path) = match paths.as_mut_slice() {
            [path] => (std::mem::take(path), None),
            [path, new_path] => (std::mem::take(path), Some(std::mem::take(new_path))),
            _ => return Err(()),
        };

        if let EventKind::Modify(ModifyKind::Name(RenameMode::To)) = kind {
            Ok(WatchEvent {
                operation: WatchEventKind::Rename,
                path: None,
                new_path: Some(path),
            })
        } else {
            Ok(WatchEvent {
                operation: kind.try_into()?,
                path: Some(path),
                new_path,
            })
        }
    }
}

struct WatchIterator {
    /// Debouncer needs to be kept alive for `rx` to keep receiving events.
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    rx: Option<Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>>,
    signals: Signals,
}

impl WatchIterator {
    fn new(
        debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
        rx: Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
        signals: Signals,
    ) -> Self {
        Self {
            _debouncer: debouncer,
            rx: Some(rx),
            signals,
        }
    }
}

impl Iterator for WatchIterator {
    type Item = Result<Vec<WatchEvent>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let rx = self.rx.as_ref()?;
        while !self.signals.interrupted() {
            let x = match rx.recv_timeout(CHECK_CTRL_C_FREQUENCY) {
                Ok(x) => x,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    self.rx = None;
                    return Some(Err(ShellError::Generic(GenericError::new_internal(
                        "Disconnected",
                        "Unexpected disconnect from file watcher",
                    ))));
                }
            };

            let Ok(events) = x else {
                self.rx = None;
                return Some(Err(ShellError::Generic(GenericError::new_internal(
                    "Receiving events failed",
                    "Unexpected errors when receiving events",
                ))));
            };

            let watch_events = events
                .into_iter()
                .filter_map(|ev| WatchEvent::try_from(ev).ok())
                .collect::<Vec<_>>();

            return Some(Ok(watch_events));
        }
        self.rx = None;
        None
    }
}
