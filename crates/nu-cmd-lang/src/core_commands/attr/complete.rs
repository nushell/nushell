use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrComplete;

impl Command for AttrComplete {
    fn name(&self) -> &str {
        "attr complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr complete")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Nothing, Type::String),
            ])
            .allow_variants_without_examples(true)
            .optional(
                "completer",
                SyntaxShape::String,
                "Name of the completion command command.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for enabling use of the external completer, or addition of a custom one, to command."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        run_impl(arg, call.span())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: Option<Spanned<String>> = call.opt_const(working_set, 0)?;
        run_impl(arg, call.span())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }
}

fn run_impl(arg: Option<Spanned<String>>, head: Span) -> Result<PipelineData, ShellError> {
    Ok(arg
        .map(|Spanned { item, span }| Value::string(item, span))
        .unwrap_or(Value::nothing(head))
        .into_pipeline_data())
}
