use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .input_output_types(vec![
                (Type::Date, Type::String),
                (Type::String, Type::String),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "lists strftime cheatsheet", Some('l'))
            .optional(
                "format string",
                SyntaxShape::String,
                "The desired date format.",
            )
            .category(Category::Removed)
    }

    fn usage(&self) -> &str {
        "Removed command: use `format date` instead."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::RemovedCommand {
            removed: self.name().to_string(),
            replacement: "format date".to_owned(),
            span: call.head,
        })
    }
}
