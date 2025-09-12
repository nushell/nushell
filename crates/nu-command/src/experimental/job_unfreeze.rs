use nu_engine::command_prelude::*;
use nu_protocol::{
    JobId,
    engine::{FrozenJob, Job, ThreadJob},
    process::check_ok,
};
use nu_system::{ForegroundWaitStatus, kill_by_pid};

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

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        let id: Option<usize> = call.opt(engine_state, stack, 0)?;
        let id = id
            .map(JobId::new)
            .or_else(|| jobs.most_recent_frozen_job_id())
            .ok_or(JobError::NoneToUnfreeze { span: head })?;

        let job = match jobs.lookup(id) {
            None => return Err(JobError::NotFound { span: head, id }.into()),
            Some(Job::Thread(ThreadJob { .. })) => {
                return Err(JobError::CannotUnfreeze { span: head, id }.into());
            }
            Some(Job::Frozen(FrozenJob { .. })) => jobs
                .remove_job(id)
                .expect("job was supposed to be in job list"),
        };

        drop(jobs);

        unfreeze_job(engine_state, id, job, head)?;

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "job unfreeze",
                description: "Unfreeze the latest frozen job",
                result: None,
            },
            Example {
                example: "job unfreeze 4",
                description: "Unfreeze a specific frozen job by its PID",
                result: None,
            },
        ]
    }

    fn extra_description(&self) -> &str {
        r#"When a running process is frozen (with the SIGTSTP signal or with the Ctrl-Z key on unix),
a background job gets registered for this process, which can then be resumed using this command."#
    }
}

fn unfreeze_job(
    state: &EngineState,
    old_id: JobId,
    job: Job,
    span: Span,
) -> Result<(), ShellError> {
    match job {
        Job::Thread(ThreadJob { .. }) => Err(JobError::CannotUnfreeze { span, id: old_id }.into()),
        Job::Frozen(FrozenJob {
            unfreeze: handle,
            tag,
        }) => {
            let pid = handle.pid();

            if let Some(thread_job) = &state.current_thread_job()
                && !thread_job.try_add_pid(pid)
            {
                kill_by_pid(pid.into()).map_err(|err| {
                    ShellError::Io(IoError::new_internal(
                        err,
                        "job was interrupted; could not kill foreground process",
                        nu_protocol::location!(),
                    ))
                })?;
            }

            let result = handle.unfreeze(
                state
                    .is_interactive
                    .then(|| state.pipeline_externals_state.clone()),
            );

            if let Some(thread_job) = &state.current_thread_job() {
                thread_job.remove_pid(pid);
            }

            match result {
                Ok(ForegroundWaitStatus::Frozen(handle)) => {
                    let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");

                    jobs.add_job_with_id(
                        old_id,
                        Job::Frozen(FrozenJob {
                            unfreeze: handle,
                            tag,
                        }),
                    )
                    .expect("job was supposed to be removed");

                    if state.is_interactive {
                        println!("\nJob {} is re-frozen", old_id.get());
                    }
                    Ok(())
                }

                Ok(ForegroundWaitStatus::Finished(status)) => check_ok(status, false, span),

                Err(err) => Err(ShellError::Io(IoError::new_internal(
                    err,
                    "Failed to unfreeze foreground process",
                    nu_protocol::location!(),
                ))),
            }
        }
    }
}
