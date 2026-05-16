use std::{borrow::Cow, sync::Arc};

use nu_engine::command_prelude::*;
use nu_utils::escape_quote_string;

use crate::completions::{Context, DirectoryCompletion, FileCompletion, NuCompleter};

#[derive(Clone)]
pub struct CommandlineComplete;

impl Command for CommandlineComplete {
    fn name(&self) -> &str {
        "commandline complete"
    }

    fn description(&self) -> &str {
        "Complete a string using the default completions."
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline complete")
            .input_output_type(
                Type::Nothing,
                Type::one_of([Type::list(Type::String), Type::list(Type::record())]),
            )
            .input_output_type(
                Type::String,
                Type::one_of([Type::list(Type::String), Type::list(Type::record())]),
            )
            .switch(
                "detailed",
                "Output completions as records, in the format expected from custom completers.",
                Some('d'),
            )
            .param(
                Flag::new("type")
                    .arg(SyntaxShape::String)
                    .desc("The type of values to allow as completions.")
                    .completion(Completion::List(nu_utils::NuCow::Borrowed(&[
                        "directory",
                        "glob",
                        "path",
                    ]))),
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        "This command can be used to obtain the completions that Nushell would normally provide for the given commandline contents.
Completions will be provided as if the cursor is placed at the end of the given string.

If no input is provided, the current commandline contents will be used instead."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive", "completion"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let detailed_completions = call.has_flag(engine_state, stack, "detailed")?;

        // TODO: it should be possible to add something like a `NuCompleter::borrowed()`
        // to avoid cloning the entire stack + engine state here, as a future optimization.
        let completer = NuCompleter::new(Arc::new(engine_state.clone()), Arc::new(stack.clone()));

        let src_span = input.span().unwrap_or(call.head);

        let (buffer, cursor_pos): (Cow<_>, _) = match &input {
            PipelineData::Empty => {
                // Clone the repl buffer to avoid holding the lock while fetching completions, which
                // may execute arbitrary code (including other `commandline` calls that access the repl state).
                let repl = engine_state.repl_state.lock().expect("repl state mutex");
                (Cow::from(repl.buffer.clone()), repl.cursor_pos)
            }
            PipelineData::Value(Value::String { val, .. }, _) => (val.as_str().into(), val.len()),
            _ => {
                return Err(ShellError::PipelineMismatch {
                    exp_input_type: "string or nothing".into(),
                    dst_span: call.head,
                    src_span,
                });
            }
        };

        let completions =
            if let Some(shape) = call.get_flag::<Value>(engine_state, stack, "type")? {
                let mut working_set = StateWorkingSet::new(engine_state);
                let span = {
                    let file = working_set.add_file("completer", buffer.as_bytes());
                    working_set.get_span_for_file(file)
                };
                let ctx = Context::new(
                    &working_set,
                    span,
                    &buffer.as_bytes()[..cursor_pos],
                    span.start,
                );

                match shape.as_str()? {
                    "directory" => completer.process_completion(&mut DirectoryCompletion, &ctx),
                    "path" | "glob" => completer.process_completion(&mut FileCompletion, &ctx),
                    other => {
                        return Err(ShellError::InvalidValue {
                            valid: r#"type "directory", "path", or "glob""#.into(),
                            actual: escape_quote_string(other),
                            span: shape.span(),
                        });
                    }
                }
            } else {
                completer.fetch_completions_at(&buffer, cursor_pos)
            };

        let result = completions
            .into_iter()
            .map(|suggestion| {
                if detailed_completions {
                    suggestion.into_value(call.span())
                } else {
                    Value::string(suggestion.suggestion.value, call.span())
                }
            })
            .collect();

        Ok(Value::list(result, call.span()).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List completions for command names.",
                example: "def my-bar [] {}; def my-baz [] {}; 'my-' | commandline complete",
                result: Some(Value::list(
                    vec![
                        Value::string("my-bar", Span::test_data()),
                        Value::string("my-baz", Span::test_data()),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "List completions for flags for a command.",
                example: "def cmd [--flag(-f): string] {}; 'cmd -' | commandline complete",
                result: Some(Value::list(
                    vec![
                        Value::string("--flag", Span::test_data()),
                        Value::string("-f", Span::test_data()),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Complete filepath or glob arguments.",
                example: "'./' | commandline complete --type path",
                result: None,
            },
            Example {
                description: "Extend builtin completions for the current commandline.",
                example: "commandline complete | append 'foo'",
                result: None,
            },
        ]
    }
}
