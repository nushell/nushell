use super::state::{WatchStreamOptions, stream_watch};
use nu_engine::command_prelude::*;
use std::time::Duration;

#[derive(Clone)]
pub struct IdxWatch;

impl Command for IdxWatch {
    fn name(&self) -> &str {
        "idx watch"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "pattern",
                SyntaxShape::String,
                "Base-relative glob, path, or directory to watch. Omit or pass empty to watch the whole indexed tree.",
            )
            .named(
                "ignore",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "List of globs or path-prefixes to exclude.",
                Some('i'),
            )
            .named(
                "timeout",
                SyntaxShape::Duration,
                "Stop streaming after this duration.",
                Some('t'),
            )
            .named(
                "max-events",
                SyntaxShape::Int,
                "Stop after emitting this many events.",
                Some('n'),
            )
            .input_output_types(vec![(
                Type::Nothing,
                Type::Table(
                    vec![
                        ("kind".into(), Type::String),
                        ("path".into(), Type::String),
                    ]
                    .into(),
                ),
            )])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Stream filesystem change events from the live idx index."
    }

    fn extra_description(&self) -> &str {
        "Requires a live runtime with watching enabled (`idx init` or `idx import` without `--no-watch`). \
Events are debounced by fff-search and emitted as records with `kind` (`created`, `modified`, `removed`, `rescan`) \
and absolute `path`. Gitignored and other index-ignored files do not produce events. \
Patterns must be inside the indexed base path. Use plain `watch` for ad-hoc path watching without an index."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["watcher", "filesystem", "notify", "events"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Watch the whole indexed tree after initializing idx.",
                example: "idx init .; idx watch",
                result: None,
            },
            Example {
                description: "Watch only Rust files, ignoring a vendor-style path prefix.",
                example: r#"idx watch "**/*.rs" --ignore [target]"#,
                result: None,
            },
            Example {
                description: "Take action on modified files in a pipeline.",
                example: r#"idx watch | where kind == "modified" | each { |e| print $"changed: ($e.path)" }"#,
                result: None,
            },
            Example {
                description: "Stop after a single event (useful in scripts).",
                example: "idx watch --max-events 1",
                result: None,
            },
            Example {
                description: "Stop after a duration if no more events are needed.",
                example: "idx watch --timeout 5sec",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Option<String> = call.opt(engine_state, stack, 0)?;
        let ignore = call
            .get_flag::<Vec<String>>(engine_state, stack, "ignore")?
            .unwrap_or_default();
        let timeout: Option<Duration> = call.get_flag(engine_state, stack, "timeout")?;
        let max_events: Option<i64> = call.get_flag(engine_state, stack, "max-events")?;

        let max_events = max_events
            .map(|n| {
                if n < 0 {
                    Err(ShellError::NeedsPositiveValue { span: call.head })
                } else {
                    Ok(n as usize)
                }
            })
            .transpose()?;

        stream_watch(WatchStreamOptions {
            pattern: pattern.unwrap_or_default(),
            ignore,
            timeout,
            max_events,
            span: call.head,
            signals: engine_state.signals().clone(),
        })
    }
}
