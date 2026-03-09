use nu_engine::command_prelude::*;
use nu_protocol::engine::FilterTag;

#[derive(Clone)]
pub struct JobFlush;

impl Command for JobFlush {
    fn name(&self) -> &str {
        "job flush"
    }

    fn description(&self) -> &str {
        "Clear this job's mailbox."
    }

    fn extra_description(&self) -> &str {
        r#"
This command removes all messages in the mailbox of the current job.
If a message is received while this command is executing, it may also be discarded.
"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job flush")
            .category(Category::Experimental)
            .named(
                "tag",
                SyntaxShape::Int,
                "Clear messages with this tag.",
                None,
            )
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
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
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let tag_arg: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "tag")?;
        if let Some(tag) = tag_arg
            && tag.item < 0
        {
            return Err(ShellError::NeedsPositiveValue { span: tag.span });
        }

        let tag_arg = tag_arg.map(|it| it.item as FilterTag);

        let mut mailbox = engine_state
            .current_job
            .mailbox
            .lock()
            .expect("failed to acquire lock");

        if tag_arg.is_some() {
            while mailbox.try_recv(tag_arg).is_ok() {}
        } else {
            mailbox.clear();
        }

        Ok(Value::nothing(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "job flush",
            description: "Clear the mailbox of the current job.",
            result: None,
        }]
    }
}
