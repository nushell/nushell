use nu_engine::command_prelude::*;
use nu_protocol::{JobId, engine::Job};
use std::time::Duration;

#[derive(Clone)]
pub struct JobWait;

impl Command for JobWait {
    fn name(&self) -> &str {
        "job wait"
    }

    fn description(&self) -> &str {
        r#"Wait for a job to complete."#
    }

    fn extra_description(&self) -> &str {
        r#"Given the id of a running job currently in the job table, this command
waits for it to complete, blocking the current thread. 

Note that this command fails if the provided job id is currently not in the job table
(as seen by `job list`), so it is not possible to wait for jobs that have already finished.   
        "#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job wait")
            .category(Category::Experimental)
            .required("id", SyntaxShape::Int, "The id of the running to wait for.")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["join"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let id_arg: Spanned<i64> = call.req(engine_state, stack, 0)?;

        if id_arg.item < 0 {
            return Err(ShellError::NeedsPositiveValue { span: id_arg.span });
        }

        let id: JobId = JobId::new(id_arg.item as usize);

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        match jobs.lookup_mut(id) {
            None => Err(ShellError::Job(JobError::NotFound { id, span: head })),

            Some(Job::Frozen { .. }) => {
                Err(ShellError::Job(JobError::AlreadyFrozen { id, span: head }))
            }

            Some(Job::Thread(job)) => {
                let waiter = job.on_termination().clone();

                // .wait() blocks so we drop our mutex guard
                drop(jobs);

                wait_with_interrupt(
                    |time| waiter.wait_timeout(time),
                    || engine_state.signals().check(&head),
                    Duration::from_millis(100),
                )?;

                Ok(Value::nothing(head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "let id = job spawn { sleep 5sec; 'hi there' }; job wait $id",
            description: "Wait for a job to complete",
            result: Some(Value::test_string("hi there")),
        }]
    }
}

pub fn wait_with_interrupt<R, E>(
    mut wait: impl FnMut(Duration) -> Option<R>,
    mut interrupted: impl FnMut() -> Result<(), E>,
    check_interval: Duration,
) -> Result<R, E> {
    loop {
        interrupted()?;

        if let Some(result) = wait(check_interval) {
            return Ok(result);
        }
    }
}
