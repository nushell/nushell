use nu_engine::command_prelude::*;
use nu_protocol::JobId;

#[derive(Clone)]
pub struct JobTag;

impl Command for JobTag {
    fn name(&self) -> &str {
        "job tag"
    }

    fn description(&self) -> &str {
        "Add a description tag to a background job."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job tag")
            .category(Category::Experimental)
            .required("id", SyntaxShape::Int, "The id of the job to tag.")
            .required(
                "tag",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                "The tag to assign to the job.",
            )
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["describe", "desc"]
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

        let tag: Option<String> = call.req(engine_state, stack, 1)?;

        let mut jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        match jobs.lookup_mut(id) {
            None => {
                return Err(ShellError::JobNotFound {
                    id: id.get(),
                    span: head,
                });
            }

            Some(job) => job.assign_tag(tag),
        }

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "let id = job spawn { sleep 10sec }; job tag $id abc ",
            description: "tag a newly spawned job",
            result: None,
        }]
    }
}
