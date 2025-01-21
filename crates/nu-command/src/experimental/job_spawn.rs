use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::{
    engine::{Closure, Job},
    report_shell_error,
};

#[derive(Clone)]
pub struct Spawn;

impl Command for Spawn {
    fn name(&self) -> &str {
        "job spawn"
    }

    fn description(&self) -> &str {
        "Spawn a background job"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job spawn")
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
        vec!["background", "bg", "&"]
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

        let mut job_state = engine_state.clone();
        job_state.is_interactive = false;

        let job_stack = stack.clone();
        
        let job_signals = engine_state.signals().clone();

        thread::spawn(move || {
            let id = {
                // TODO: proper mutex error handling
                let mut jobs = job_state.jobs.lock().unwrap();

                jobs.add_job(Job {
                    signals: job_signals,
                })
            };

            ClosureEvalOnce::new(&job_state, &job_stack, closure.clone())
                .run_with_input(Value::nothing(head).into_pipeline_data())
                .and_then(|data| data.into_value(head))
                .unwrap_or_else(|err| {
                    report_shell_error(&job_state, &err);

                    Value::nothing(head)
                });

            {
                let mut jobs = job_state.jobs.lock().unwrap();

                jobs.unregister_job(id)
            }
        });

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
