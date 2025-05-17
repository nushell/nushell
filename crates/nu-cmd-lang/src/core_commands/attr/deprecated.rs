use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrDeprecated;

impl Command for AttrDeprecated {
    fn name(&self) -> &str {
        "attr deprecated"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr deprecated")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Nothing, Type::String),
            ])
            .optional(
                "message",
                SyntaxShape::String,
                "Message to include with deprecation warning.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for marking a command as deprecated."
    }

    fn extra_description(&self) -> &str {
        "Also consider setting the category to deprecated with @category deprecated"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let message: Option<Value> = call.opt(engine_state, stack, 0)?;
        match message {
            Some(message) => Ok(message.into_pipeline_data()),
            None => Ok(PipelineData::Empty),
        }
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let message: Option<Value> = call.opt_const(working_set, 0)?;
        match message {
            Some(message) => Ok(message.into_pipeline_data()),
            None => Ok(PipelineData::Empty),
        }
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Add a deprecation warning to a custom command",
                example: r###"@deprecated
    def outdated [] {}"###,
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Add a deprecation warning with a custom message",
                example: r###"@deprecated "Use my-new-command instead."
    @category deprecated
    def my-old-command [] {}"###,
                result: Some(Value::string(
                    "Use my-new-command instead.",
                    Span::test_data(),
                )),
            },
        ]
    }
}
