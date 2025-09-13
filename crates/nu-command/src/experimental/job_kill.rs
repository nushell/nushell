use nu_engine::command_prelude::*;
use nu_protocol::JobId;

#[derive(Clone)]
pub struct JobKill;

impl Command for JobKill {
    fn name(&self) -> &str {
        "job kill"
    }

    fn description(&self) -> &str {
        "Kill a background job."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job kill")
            .category(Category::Experimental)
            .required("id", SyntaxShape::Int, "The id of the job to kill.")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["halt", "stop", "end", "close"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let id_arg: Spanned<usize> = call.req(engine_state, stack, 0)?;
        let id = JobId::new(id_arg.item);

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        if jobs.lookup(id).is_none() {
            return Err(JobError::NotFound { span: head, id }.into());
        }

        jobs.kill_and_remove(id).map_err(|err| {
            ShellError::Io(IoError::new_internal(
                err,
                "Failed to kill the requested job",
                nu_protocol::location!(),
            ))
        })?;

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "let id = job spawn { sleep 10sec }; job kill $id",
            description: "Kill a newly spawned job",
            result: None,
        }]
    }
}
