use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrSearchTerms;

impl Command for AttrSearchTerms {
    fn name(&self) -> &str {
        "attr search-terms"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr search-terms")
            .input_output_type(Type::Nothing, Type::list(Type::String))
            .allow_variants_without_examples(true)
            .rest("terms", SyntaxShape::String, "Search terms.")
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for adding search terms to custom commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = call.rest(engine_state, stack, 0)?;
        Ok(Value::list(args, call.head).into_pipeline_data())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = call.rest_const(working_set, 0)?;
        Ok(Value::list(args, call.head).into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Add search terms to a custom command",
            example: r###"# Double numbers
    @search-terms multiply times
    def double []: [number -> number] { $in * 2 }"###,
            result: None,
        }]
    }
}
