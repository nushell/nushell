use nu_engine::command_prelude::*;
use nu_protocol::{engine::Tag, JobId};

#[derive(Clone)]
pub struct JobSend;

impl Command for JobSend {
    fn name(&self) -> &str {
        "job send"
    }

    fn description(&self) -> &str {
        "Send a message to the mailbox of a job."
    }

    fn extra_description(&self) -> &str {
        r#"
This command sends a message to a background job, which can then read sent messages
in a first-in-first-out fashion with `job recv`. When it does so, it may additionally specify a numeric filter tag,
in which case it will only read messages sent with the exact same filter tag. 
In particular, the id 0 refers to the main/initial nushell thread.

A message can be any nushell value, and streams are always collected before being sent.

This command never blocks.
"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job send")
            .category(Category::Experimental)
            .required(
                "id",
                SyntaxShape::Int,
                "The id of the job to send the message to.",
            )
            .named("tag", SyntaxShape::Int, "A tag for the message", None)
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let id_arg: Spanned<i64> = call.req(engine_state, stack, 0)?;
        let tag_arg: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "tag")?;

        let id = id_arg.item;

        if id < 0 {
            return Err(ShellError::NeedsPositiveValue { span: id_arg.span });
        }

        if let Some(tag) = tag_arg {
            if tag.item < 0 {
                return Err(ShellError::NeedsPositiveValue { span: tag.span });
            }
        }

        let tag = tag_arg.map(|it| it.item as Tag);

        let value = input.into_value(head)?;

        if id == 0 {
            engine_state
                .root_job_sender
                .send((tag, value))
                .expect("this should NEVER happen.");
        } else {
            let jobs = engine_state.jobs.lock().expect("failed to acquire lock");

            if let Some(job) = jobs.lookup(JobId::new(id as usize)) {
                match job {
                    nu_protocol::engine::Job::Thread(thread_job) => {
                        // it is ok to send this value while holding the lock, because
                        // mail channels are always unbounded, so this send never blocks
                        let _ = thread_job.sender.send((tag, value));
                    }
                    nu_protocol::engine::Job::Frozen(_) => {
                        return Err(ShellError::NeedThreadJob {
                            id: id as usize,
                            span: id_arg.span,
                        });
                    }
                }
            } else {
                return Err(ShellError::JobNotFound {
                    id: id as usize,
                    span: id_arg.span,
                });
            }
        }

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "let id = job spawn { job recv | save sent.txt }; 'hi' | job send $id",
            description: "Send a message to a newly spawned job",
            result: None,
        }]
    }
}
