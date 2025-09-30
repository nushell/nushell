use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrComplete;

impl Command for AttrComplete {
    fn name(&self) -> &str {
        "attr complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr complete")
            .input_output_type(Type::Nothing, Type::String)
            .allow_variants_without_examples(true)
            .required(
                "completer",
                SyntaxShape::String,
                "Name of the completion command command.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for adding a custom completer that acts on the whole command."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Spanned<String> = call.req(engine_state, stack, 0)?;
        run_impl(arg)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Spanned<String> = call.req_const(working_set, 0)?;
        run_impl(arg)
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }
}

fn run_impl(Spanned { item, span }: Spanned<String>) -> Result<PipelineData, ShellError> {
    Ok(Value::string(item, span).into_pipeline_data())
}

#[derive(Clone)]
pub struct AttrCompleteExternal;

impl Command for AttrCompleteExternal {
    fn name(&self) -> &str {
        "attr complete external"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr complete external")
            .input_output_type(Type::Nothing, Type::Nothing)
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for enabling use of the external completer for internal commands."
    }

    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        _: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn run_const(
        &self,
        _: &StateWorkingSet,
        _: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }
}
