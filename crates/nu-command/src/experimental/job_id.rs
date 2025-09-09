use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct JobId;

impl Command for JobId {
    fn name(&self) -> &str {
        "job id"
    }

    fn description(&self) -> &str {
        "Get id of current job."
    }

    fn extra_description(&self) -> &str {
        "This command returns the job id for the current background job. 
The special id 0 indicates that this command was not called from a background job thread, and 
was instead spawned by main nushell execution thread."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job id")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Int)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["self", "this", "my-id", "this-id"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        Ok(Value::int(engine_state.current_job.id.get() as i64, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "job id",
            description: "Get id of current job",
            result: None,
        }]
    }
}
