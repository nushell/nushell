use std::sync::Arc;

use nu_engine::command_prelude::*;

use crate::NuCompleter;

#[derive(Clone)]
pub struct CommandlineComplete;

impl Command for CommandlineComplete {
    fn name(&self) -> &str {
        "commandline complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Complete a string using default completions"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive", "completion"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // HACK: this should use the one that already exists in repl.rs instead of cloning everything
        let completer = NuCompleter::new(Arc::new(engine_state.clone()), Arc::new(stack.clone()));

        let completions = match &input {
            PipelineData::Empty => {
                let repl = engine_state.repl_state.lock().expect("repl state mutex");
                completer.fetch_completions_at(&repl.buffer, repl.cursor_pos)
            }
            PipelineData::Value(Value::String { val, .. }, _) => {
                completer.fetch_completions_at(&val, val.len())
            }
            _ => todo!(),
        };

        Ok(Value::list(
            completions
                .into_iter()
                .map(|completion| {
                    // TODO: convert to record like crate::completions::map_value_completion expects?
                    Value::string(
                        completion.suggestion.value,
                        Span {
                            // TODO may need some offset here?
                            start: completion.suggestion.span.start,
                            end: completion.suggestion.span.end,
                        },
                    )
                })
                .collect(),
            Span::unknown(),
        )
        .into_pipeline_data())
    }
}
