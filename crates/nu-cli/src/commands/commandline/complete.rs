use std::sync::Arc;

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

        let working_set = StateWorkingSet::new(engine_state);

        // TODO: it should be possible to add something like a `NuCompleter::borrowed()`
        // to avoid cloning the entire stack + engine state here, as a future optimization.
        let completer = NuCompleter::new(Arc::new(engine_state.clone()), Arc::new(stack.clone()));

        let repl = engine_state.repl_state.lock().expect("repl state mutex");
        let (buf, pos, input_span) = match &input {
            PipelineData::Empty => (repl.buffer.as_str(), repl.cursor_pos, Span::unknown()),
            PipelineData::Value(v @ Value::String { val, .. }, _) => {
                (val.as_str(), val.len(), v.span())
            }
            input => {
                return Err(ShellError::PipelineMismatch {
                    exp_input_type: "string or nothing".into(),
                    dst_span: call.head,
                    src_span: input.span().unwrap_or(call.head),
                });
            }
        };

        let completions =
            if let Some(shape) = call.get_flag::<Value>(engine_state, stack, "type")? {
                // Completion internals use the span/offset to determine if a completion is "intermediate"
                // to limit to directories, but for our purposes we always want all completions,
                // so we make a new span to sidestep that logic.
                //
                // We still retain the original span's start, so that the resulting
                // completions have the same offset as if we had used the input span.
                let offset = input_span.start;
                let span = Span::new(offset, offset + buf.len());
                let ctx = Context::new(&working_set, span, buf.as_bytes(), offset);

                match shape.as_str()? {
                    "directory" => completer.process_completion(&mut DirectoryCompletion, &ctx),
                    "path" | "glob" => completer.process_completion(&mut FileCompletion, &ctx),
                    actual => {
                        return Err(ShellError::InvalidValue {
                            valid: r#"type "directory", "path", or "glob""#.into(),
                            actual: escape_quote_string(actual),
                            span: shape.span(),
                        });
                    }
                }
            } else {
                completer.fetch_completions_at(buf, pos)
            };

        drop(repl);

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
