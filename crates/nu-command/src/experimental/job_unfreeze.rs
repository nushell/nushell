use nu_engine::command_prelude::*;
use nu_engine::{FrozenPipelineState, PipelineProxy};
use nu_protocol::{
    JobId, ListStream, Signals,
    engine::{FrozenJob, Job, ThreadJob},
    process::check_ok,
};
use nu_system::{ForegroundWaitStatus, UnfreezeHandle, kill_by_pid};

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
            .input_output_types(vec![(Type::Nothing, Type::Any)])
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

        unfreeze_job(engine_state, id, job, head)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "job unfreeze",
                description: "Unfreeze the latest frozen job.",
                result: None,
            },
            Example {
                example: "job unfreeze 4",
                description: "Unfreeze a specific frozen job by its PID.",
                result: None,
            },
        ]
    }

    fn extra_description(&self) -> &str {
        "When a running process is frozen (with the SIGTSTP signal or with the Ctrl-Z key on unix),
a background job gets registered for this process, which can then be resumed using this command."
    }
}

fn unfreeze_job(
    state: &EngineState,
    old_id: JobId,
    job: Job,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match job {
        Job::Thread(ThreadJob { .. }) => Err(JobError::CannotUnfreeze { span, id: old_id }.into()),
        Job::Frozen(FrozenJob {
            unfreeze: handle,
            description,
            pipeline_state,
        }) => {
            // Thread-based pipeline jobs are handled directly — they don't go through the
            // external-process wait loop.
            if matches!(handle, UnfreezeHandle::Thread { .. }) {
                return unfreeze_thread_job(state, handle, pipeline_state, span);
            }

            // External process job (pid > 0).
            let pid = handle.pid();

            if pid > 0
                && let Some(thread_job) = &state.current_thread_job()
                && !thread_job.try_add_pid(pid)
            {
                kill_by_pid(pid.into()).map_err(|err| {
                    ShellError::Io(IoError::new_internal(
                        err,
                        "job was interrupted; could not kill foreground process",
                    ))
                })?;
            }

            let result = handle.unfreeze(
                state
                    .is_interactive
                    .then(|| state.pipeline_externals_state.clone()),
            );

            if pid > 0
                && let Some(thread_job) = &state.current_thread_job()
            {
                thread_job.remove_pid(pid);
            }

            match result {
                Ok(ForegroundWaitStatus::Frozen(handle)) => {
                    let mut jobs = state.jobs.lock().expect("jobs lock is poisoned!");

                    jobs.add_job_with_id(
                        old_id,
                        Job::Frozen(FrozenJob {
                            unfreeze: handle,
                            description,
                            pipeline_state: None,
                        }),
                    )
                    .expect("job was supposed to be removed");

                    if state.is_interactive {
                        println!("\nJob {} is re-frozen", old_id.get());
                    }
                    Ok(PipelineData::Empty)
                }

                Ok(ForegroundWaitStatus::Finished(status)) => {
                    check_ok(status, false, span)?;
                    Ok(PipelineData::Empty)
                }

                Err(err) => Err(ShellError::Io(IoError::new_internal(
                    err,
                    "Failed to unfreeze foreground process",
                ))),
            }
        }
    }
}

/// Resume a thread-based pipeline job, returning any remaining stream output.
fn unfreeze_thread_job(
    state: &EngineState,
    handle: UnfreezeHandle,
    pipeline_state: Option<Box<dyn std::any::Any + Send>>,
    _span: Span,
) -> Result<PipelineData, ShellError> {
    // Resume the worker (if it is parked at a cooperative yield point).
    if let UnfreezeHandle::Thread {
        ref suspend_state, ..
    } = handle
    {
        suspend_state.resume();
    }

    // If we have a frozen stream state, reconstruct the proxy and return the remaining output.
    let frozen_state = pipeline_state
        .and_then(|b| b.downcast::<FrozenPipelineState>().ok())
        .map(|b| *b);

    if let Some(state_data) = frozen_state {
        let span = state_data.span;
        let metadata = state_data.metadata.clone();
        let proxy = PipelineProxy::new(state_data, state.jobs.clone(), state.is_interactive);
        let stream = ListStream::new(proxy, span, Signals::empty());
        Ok(PipelineData::list_stream(stream, metadata))
    } else {
        Ok(PipelineData::Empty)
    }
}
