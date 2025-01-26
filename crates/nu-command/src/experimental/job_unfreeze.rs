use nu_engine::command_prelude::*;
use nu_protocol::{
    engine::{Job, JobId},
    process::check_ok,
};
use nu_system::ForegroundWaitStatus;

#[derive(Clone)]
pub struct JobUnfreeze;

impl Command for JobUnfreeze {
    fn name(&self) -> &str {
        "job unfreeze"
    }

    fn description(&self) -> &str {
        "Unfreeze a frozen process job in foreground."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job unfreeze")
            .category(Category::Experimental)
            // TODO: make this argument optional and use highest most recent job if
            // no argument is passed
            .required("id", SyntaxShape::Int, "The process id to kill")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fg"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let id: i64 = call.req(engine_state, stack, 0)?;

        let id: JobId = id as JobId;

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        // FIXME: this is broken; when given a thread job, it is removed from the queue.

        let job = match jobs.remove_job(id) {
            None => return Err(ShellError::NotFound { span: head }),
            Some(job) => job,
        };

        drop(jobs);

        unfreeze_job(engine_state, job, head)?;

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn unfreeze_job(state: &EngineState, job: Job, span: Span) -> Result<(), ShellError> {
    match job {
        Job::ThreadJob { .. } => {
            // TODO: add new ShellError for this
            Err(ShellError::IncompatibleParametersSingle {
                msg: "i cannot unfreeze a thread job".into(),
                span,
            })
        }

        Job::FrozenJob { unfreeze } => match unfreeze.unfreeze_in_foreground()? {
            ForegroundWaitStatus::Frozen(unfreeze) => {
                let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");
                jobs.add_job(Job::FrozenJob { unfreeze });
                Ok(())
            }

            ForegroundWaitStatus::Finished(status) => check_ok(status, false, span),
        },
    }
}
