use nu_engine::command_prelude::*;
use nu_protocol::JobId;

#[derive(Clone)]
pub struct JobDescribe;

impl Command for JobDescribe {
    fn name(&self) -> &str {
        "job describe"
    }

    fn description(&self) -> &str {
        "Add a description to a background job."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job describe")
            .category(Category::Experimental)
            .required("id", SyntaxShape::Int, "The id of the job to describe.")
            .required(
                "description",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                "The description to assign to the job.",
            )
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["describe", "tag", "name"]
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

        let description: Option<String> = call.req(engine_state, stack, 1)?;

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        match jobs.lookup_mut(id) {
            None => return Err(JobError::NotFound { span: head, id }.into()),
            Some(job) => job.assign_description(description),
        }

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "let id = job spawn { sleep 10sec }; job describe $id abc ",
                description: "Describe a newly spawned job",
                result: None,
            },
            Example {
                example: "let id = job spawn { sleep 10sec }; job describe $id abc; job describe $id null",
                description: "Remove the description of a job",
                result: None,
            },
        ]
    }
}
