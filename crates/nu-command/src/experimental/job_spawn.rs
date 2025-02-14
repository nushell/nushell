use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
    thread,
};

use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::{
    engine::{Closure, Job, ThreadJob},
    report_shell_error, Signals,
};

#[derive(Clone)]
pub struct JobSpawn;

impl Command for JobSpawn {
    fn name(&self) -> &str {
        "job spawn"
    }

    fn description(&self) -> &str {
        "Spawn a background job."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job spawn")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run in another thread.",
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

        // the new job should have its ctrl-c independent of foreground
        let job_signals = Signals::new(Arc::new(AtomicBool::new(false)));
        job_state.set_signals(job_signals.clone());

        // the new job has a separate process group state for its processes
        job_state.pipeline_externals_state = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));

        job_state.exit_warning_given = Arc::new(AtomicBool::new(false));

        thread::spawn(move || {
            let id = {
                let mut jobs = job_state.jobs.lock().expect("jobs lock is poisoned!");

                let thread_job = ThreadJob::new(job_signals);

                job_state.current_thread_job = Some(thread_job.clone());
                jobs.add_job(Job::Thread(thread_job))
            };

            ClosureEvalOnce::new(&job_state, &job_stack, closure.clone())
                .run_with_input(Value::nothing(head).into_pipeline_data())
                .and_then(|data| data.into_value(head))
                .unwrap_or_else(|err| {
                    if !job_state.signals().interrupted() {
                        report_shell_error(&job_state, &err);
                    }

                    Value::nothing(head)
                });

            {
                let mut jobs = job_state.jobs.lock().expect("jobs lock is poisoned!");

                jobs.remove_job(id);
            }
        });

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
