use std::thread;

use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Spawn;

impl Command for Spawn {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn a background job"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("spawn")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run in another thread",
            )
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["spawn", "job", "background", "ampersand"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let closure: Closure = call.req(engine_state, stack, 0)?;

        let mut cloned_state = engine_state.clone();
        let cloned_stack = stack.clone();

        thread::spawn(move || {
            cloned_state.is_interactive = false;

            ClosureEvalOnce::new(&cloned_state, &cloned_stack, closure.clone())
                .run_with_input(Value::nothing(head).into_pipeline_data())
                .and_then(|data| data.into_value(head))
                .unwrap_or_else(|err| {
                    // TODO: display this error value

                    Value::error(err, head)
                });
        });

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
