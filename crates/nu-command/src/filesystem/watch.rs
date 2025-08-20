use itertools::{Either, Itertools};
use notify_debouncer_full::{
    DebouncedEvent, Debouncer, FileIdMap, new_debouncer,
    notify::{
        self, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
        event::{DataChange, ModifyKind, RenameMode},
    },
};
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{
    DeprecationEntry, DeprecationType, ReportMode, Signals, engine::Closure, report_shell_error,
    shell_error::io::IoError,
};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
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

    fn extra_description(&self) -> &str {
        "When run without a closure, `watch` returns a stream of events instead."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["watcher", "reload", "filesystem"]
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![DeprecationEntry {
            ty: DeprecationType::Flag("debounce-ms".into()),
            report_mode: ReportMode::FirstUse,
            since: Some("0.107.0".into()),
            expected_removal: Some("0.109.0".into()),
            help: Some("`--debounce-ms` will be removed in favour of  `--debounce`".into()),
        }]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("watch")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (
                    Type::Nothing,
                    Type::Table(vec![
                        ("operation".into(), Type::String),
                        ("path".into(), Type::String),
                        ("new_path".into(), Type::String),
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
                "debounce-ms",
                SyntaxShape::Int,
                "Debounce changes for this many milliseconds (default: 100). Adjust if you find that single writes are reported as multiple events (deprecated)",
                Some('d'),
            )
            .named(
                "debounce",
                SyntaxShape::Duration,
                "Debounce changes for this duration (default: 100ms). Adjust if you find that single writes are reported as multiple events",
                None,
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

        let path_no_whitespace = path_arg
            .item
            .trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

        let path = nu_path::canonicalize_with(path_no_whitespace, cwd).map_err(|err| {
            ShellError::Io(IoError::new(
                err,
                path_arg.span,
                PathBuf::from(path_no_whitespace),
            ))
        })?;

        let closure: Option<Closure> = call.opt(engine_state, stack, 1)?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let quiet = call.has_flag(engine_state, stack, "quiet")?;
        let debounce_duration: Duration = resolve_duration_arguments(
            call.get_flag(engine_state, stack, "debounce-ms")?,
            call.get_flag(engine_state, stack, "debounce")?,
        )?;

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

        let mut debouncer =
            new_debouncer(debounce_duration, None, tx).map_err(|err| ShellError::GenericError {
                error: "Failed to create watcher".to_string(),
                msg: err.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;

        if let Err(err) = debouncer.watcher().watch(&path, recursive_mode) {
            return Err(ShellError::GenericError {
                error: "Failed to create watcher".to_string(),
                msg: err.to_string(),
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

        let iter = WatchIterator::new(debouncer, rx, engine_state.signals().clone());

        if let Some(closure) = closure {
            let mut closure = ClosureEval::new(engine_state, stack, closure);

            for events in iter {
                for event in events? {
                    let matches_glob = match &glob_pattern {
                        Some(glob) => glob.matches_path(&event.path),
                        None => true,
                    };
                    if verbose && glob_pattern.is_some() {
                        eprintln!("Matches glob: {matches_glob}");
                    }

                    if matches_glob {
                        let result = closure
                            .add_arg(event.operation.into_value(head))
                            .add_arg(event.path.to_string_lossy().into_value(head))
                            .add_arg(
                                event
                                    .new_path
                                    .as_deref()
                                    .map(Path::to_string_lossy)
                                    .into_value(head),
                            )
                            .run_with_input(PipelineData::empty());

                        match result {
                            Ok(val) => val.print_table(engine_state, stack, false, false)?,
                            Err(err) => report_shell_error(engine_state, &err),
                        };
                    }
                }
            }

            Ok(PipelineData::empty())
        } else {
            fn glob_filter(glob: Option<&nu_glob::Pattern>, path: &Path) -> bool {
                let Some(glob) = glob else { return true };
                glob.matches_path(path)
            }

            let out = iter
                .flat_map(|e| match e {
                    Ok(events) => Either::Right(events.into_iter().map(Ok)),
                    Err(err) => Either::Left(std::iter::once(Err(err))),
                })
                .filter_map(move |e| match e {
                    Ok(ev) => glob_filter(glob_pattern.as_ref(), &ev.path)
                        .then(|| WatchEventRecord::from(&ev).into_value(head)),
                    Err(err) => Some(Value::error(err, head)),
                })
                .into_pipeline_data(head, engine_state.signals().clone());
            Ok(out)
        }
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
                description: "Print file changes with a debounce time of 5 minutes",
                example: r#"watch /foo/bar --debounce 5min { |op, path| $"Registered ($op) on ($path)" | print }"#,
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

fn resolve_duration_arguments(
    debounce_duration_flag_ms: Option<Spanned<i64>>,
    debounce_duration_flag: Option<Spanned<Duration>>,
) -> Result<Duration, ShellError> {
    match (debounce_duration_flag, debounce_duration_flag_ms) {
        (None, None) => Ok(DEFAULT_WATCH_DEBOUNCE_DURATION),
        (Some(l), Some(r)) => Err(ShellError::IncompatibleParameters {
            left_message: "Here".to_string(),
            left_span: l.span,
            right_message: "and here".to_string(),
            right_span: r.span,
        }),
        (None, Some(val)) => match u64::try_from(val.item) {
            Ok(v) => Ok(Duration::from_millis(v)),
            Err(_) => Err(ShellError::TypeMismatch {
                err_message: "Debounce duration is invalid".to_string(),
                span: val.span,
            }),
        },
        (Some(v), None) => Ok(v.item),
    }
}

struct WatchEvent {
    operation: &'static str,
    path: PathBuf,
    new_path: Option<PathBuf>,
}

#[derive(IntoValue)]
struct WatchEventRecord<'a> {
    operation: &'static str,
    path: Cow<'a, str>,
    new_path: Option<Cow<'a, str>>,
}

impl<'a> From<&'a WatchEvent> for WatchEventRecord<'a> {
    fn from(value: &'a WatchEvent) -> Self {
        Self {
            operation: value.operation,
            path: value.path.to_string_lossy(),
            new_path: value.new_path.as_deref().map(Path::to_string_lossy),
        }
    }
}

impl TryFrom<DebouncedEvent> for WatchEvent {
    type Error = ();

    fn try_from(mut ev: DebouncedEvent) -> Result<Self, Self::Error> {
        // TODO: Maybe we should handle all event kinds?
        match ev.event.kind {
            EventKind::Create(_) => ev.paths.pop().map(|p| WatchEvent {
                operation: "Create",
                path: p,
                new_path: None,
            }),
            EventKind::Remove(_) => ev.paths.pop().map(|p| WatchEvent {
                operation: "Remove",
                path: p,
                new_path: None,
            }),
            EventKind::Modify(
                ModifyKind::Data(DataChange::Content | DataChange::Any) | ModifyKind::Any,
            ) => ev.paths.pop().map(|p| WatchEvent {
                operation: "Write",
                path: p,
                new_path: None,
            }),
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => ev
                .paths
                .drain(..)
                .rev()
                .next_array()
                .map(|[from, to]| WatchEvent {
                    operation: "Rename",
                    path: from,
                    new_path: Some(to),
                }),
            _ => None,
        }
        .ok_or(())
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
                    return Some(Err(ShellError::GenericError {
                        error: "Disconnected".to_string(),
                        msg: "Unexpected disconnect from file watcher".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    }));
                }
            };

            let Ok(events) = x else {
                self.rx = None;
                return Some(Err(ShellError::GenericError {
                    error: "Receiving events failed".to_string(),
                    msg: "Unexpected errors when receiving events".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                }));
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
