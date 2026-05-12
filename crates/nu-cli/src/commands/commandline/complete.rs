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
            .input_output_types(vec![
                (Type::Nothing, Type::List(Box::new(Type::String))),
                (Type::String, Type::List(Box::new(Type::String))),
            ])
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
        let (buf, pos) = match &input {
            PipelineData::Empty => (repl.buffer.as_str(), repl.cursor_pos),
            PipelineData::Value(Value::String { val, .. }, _) => (val.as_str(), val.len()),
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
                let offset = input.span().map(|span| span.start).unwrap_or(0);

                // Completion internals use this to determine if a completion is "intermediate"
                // to limit to directories, but for our purposes we always want all completions,
                // so we make a new span to avoid that logic.
                let span = Span::new(offset, offset + buf.chars().count());
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
