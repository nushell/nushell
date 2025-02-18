use nu_engine::command_prelude::*;
use nu_protocol::{
    engine::{FrozenJob, Job, JobId, ThreadJob},
    process::check_ok,
    shell_error,
};
use nu_system::{kill_by_pid, ForegroundWaitStatus};

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
            Some(Job::Thread(ThreadJob { .. })) => {
                return Err(ShellError::JobNotFrozen { id, span: head })
            }
            Some(Job::Frozen(FrozenJob { .. })) => jobs
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
        Job::Thread(ThreadJob { .. }) => Err(ShellError::JobNotFrozen { id: old_id, span }),

        Job::Frozen(FrozenJob { unfreeze: handle }) => {
            let pid = handle.pid();

            if let Some(thread_job) = &state.current_thread_job {
                if !thread_job.try_add_pid(pid) {
                    kill_by_pid(pid.into()).map_err(|err| {
                        ShellError::Io(IoError::new_internal(
                            err.kind(),
                            "job was interrupted; could not kill foreground process",
                            nu_protocol::location!(),
                        ))
                    })?;
                }
            }

            let result = handle.unfreeze(
                state
                    .is_interactive
                    .then(|| state.pipeline_externals_state.clone()),
            );

            if let Some(thread_job) = &state.current_thread_job {
                thread_job.remove_pid(pid);
            }

            match result {
                Ok(ForegroundWaitStatus::Frozen(handle)) => {
                    let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");

                    jobs.add_job_with_id(old_id, Job::Frozen(FrozenJob { unfreeze: handle }))
                        .expect("job was supposed to be removed");

                    Ok(())
                }

                Ok(ForegroundWaitStatus::Finished(status)) => check_ok(status, false, span),

                Err(err) => Err(ShellError::Io(IoError::new_internal(
                    shell_error::io::ErrorKind::Std(err.kind()),
                    "Failed to unfreeze foreground process",
                    nu_protocol::location!(),
                ))),
            }
        }
    }
}
