use nu_engine::command_prelude::*;
use nu_protocol::{
    engine::{Job, JobId},
    process::check_ok,
    shell_error,
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
            .optional("id", SyntaxShape::Int, "The process id to unfreeze.")
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

        let option_id: Option<i64> = call.opt(engine_state, stack, 0)?;

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        let id = option_id
            .map(|it| it as JobId)
            .or_else(|| jobs.most_recent_frozen_job_id())
            .ok_or_else(|| ShellError::NoFrozenJob { span: head })?;

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

        Job::FrozenJob { unfreeze } => match unfreeze.unfreeze_in_foreground() {
            Ok(ForegroundWaitStatus::Frozen(unfreeze)) => {
                let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");
                jobs.add_job_with_id(old_id, Job::FrozenJob { unfreeze })
                    .expect("job was supposed to be removed");
                Ok(())
            }

            Ok(ForegroundWaitStatus::Finished(status)) => check_ok(status, false, span),

            Err(err) => Err(ShellError::Io(IoError::new_internal(
                shell_error::io::ErrorKind::Std(err.kind()),
                "Failed to unfreeze foreground process",
                nu_protocol::location!(),
            ))),
        },
    }
}
