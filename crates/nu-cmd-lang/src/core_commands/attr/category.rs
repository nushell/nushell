use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrCategory;

impl Command for AttrCategory {
    fn name(&self) -> &str {
        "attr category"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr category")
            .input_output_type(Type::Nothing, Type::list(Type::String))
            .allow_variants_without_examples(true)
            .required(
                "category",
                SyntaxShape::String,
                "Category of the custom command.",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for adding a category to custom commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: String = call.req(engine_state, stack, 0)?;
        Ok(Value::string(arg, call.head).into_pipeline_data())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg: String = call.req_const(working_set, 0)?;
        Ok(Value::string(arg, call.head).into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Add a category to a custom command",
            example: r###"# Double numbers
    @category math
    def double []: [number -> number] { $in * 2 }"###,
            result: None,
        }]
    }
}
