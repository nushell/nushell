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
            .required("id", SyntaxShape::Int, "The process id to unfreeze.")
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


        let job = match jobs.lookup(id) {
            None => return Err(ShellError::JobNotFound { id, span: head }),
            Some(Job::ThreadJob { .. }) => return Err(ShellError::JobNotFrozen { id, span: head }),
            Some(Job::FrozenJob { .. }) => jobs
                .remove_job(id)
                .expect("job was supposed to be in job list"),
        };

        drop(jobs);

        unfreeze_job(engine_state, id, job, head)?;

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn unfreeze_job(
    state: &EngineState,
    old_id: JobId,
    job: Job,
    span: Span,
) -> Result<(), ShellError> {
    match job {
        Job::ThreadJob { .. } => Err(ShellError::JobNotFrozen { id: old_id, span }),

        Job::FrozenJob { unfreeze } => match unfreeze.unfreeze_in_foreground()? {
            ForegroundWaitStatus::Frozen(unfreeze) => {
                let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");
                jobs.add_job_with_id(old_id, Job::FrozenJob { unfreeze })
                    .expect("job was supposed to be removed");
                Ok(())
            }

            ForegroundWaitStatus::Finished(status) => check_ok(status, false, span),
        },
    }
}
