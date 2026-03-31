use std::sync::Arc;

use nu_engine::command_prelude::*;

use crate::NuCompleter;

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
            .input_output_types(vec![
                (Type::Nothing, Type::List(Box::new(Type::String))),
                (Type::String, Type::List(Box::new(Type::String))),
            ])
            .switch(
                "detailed",
                "Output completions as records, in the format expected from custom completers.",
                Some('d'),
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

        // TODO: should we try to reuse the repl's completer, or at least avoid cloning everything here?
        let completer = NuCompleter::new(Arc::new(engine_state.clone()), Arc::new(stack.clone()));

        let completions = match &input {
            PipelineData::Empty => {
                let repl = engine_state.repl_state.lock().expect("repl state mutex");
                completer.fetch_completions_at(&repl.buffer, repl.cursor_pos)
            }
            PipelineData::Value(Value::String { val, .. }, _) => {
                completer.fetch_completions_at(&val, val.len())
            }
            input => {
                return Err(ShellError::PipelineMismatch {
                    exp_input_type: "string or nothing".into(),
                    dst_span: call.head,
                    src_span: input.span().unwrap_or(call.head),
                });
            }
        };

        let result = completions
            .into_iter()
            .map(|suggestion| {
                let span = Span {
                    // TODO may need some offset here?
                    start: suggestion.suggestion.span.start,
                    end: suggestion.suggestion.span.end,
                };

                if detailed_completions {
                    suggestion.into_value(span)
                } else {
                    Value::string(suggestion.suggestion.value, span)
                }
            })
            .collect();

        Ok(Value::list(result, Span::unknown()).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List completions for command names.",
                example: "def bar [] {}; def baz [] {}; 'ba' | commandline complete",
                result: Some(Value::list(
                    vec![
                        Value::string("bar", Span::test_data()),
                        Value::string("baz", Span::test_data()),
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
                example: "def paths [p:path] {}; 'paths ./' | commandline complete",
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
