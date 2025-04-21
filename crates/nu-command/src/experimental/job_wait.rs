use nu_engine::command_prelude::*;
use nu_protocol::{engine::Job, JobId};

#[derive(Clone)]
pub struct JobWait;

impl Command for JobWait {
    fn name(&self) -> &str {
        "job wait"
    }

    fn description(&self) -> &str {
        r#"Wait for a job to complete and return its result value.

        Given the id of a running job currently in the job table, this command
        waits for it to complete and returns the value returned
        by the closure passed down to `job spawn`.

        Note that this command fails if a job is currently not in the job table
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
            None => {
                return Err(ShellError::JobNotFound {
                    id: id.get(),
                    span: head,
                });
            }

            Some(Job::Frozen { .. }) => {
                return Err(ShellError::UnsupportedJobType {
                    id: id.get() as usize,
                    span: head,
                    kind: "frozen".to_string(),
                });
            }

            Some(Job::Thread(job)) => {
                let waiter = job.on_termination().clone();

                // .wait() blocks so we drop our mutex guard
                drop(jobs);

                let result = waiter.wait().clone().with_span(head);

                Ok(result.into_pipeline_data())
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
