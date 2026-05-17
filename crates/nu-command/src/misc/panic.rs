use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Panic;

impl Command for Panic {
    fn name(&self) -> &str {
        "panic"
    }

    fn description(&self) -> &str {
        "Causes nushell to panic."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["crash", "throw"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("panic")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .optional(
                "msg",
                SyntaxShape::String,
                "The custom message for the panic.",
            )
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let maybe_msg: String = call
            .opt(engine_state, stack, 0)?
            .unwrap_or("Panic!".to_string());
        panic!("{}", maybe_msg)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Panic with a custom message",
            example: "panic 'This is a custom panic message'",
            result: None,
        }]
    }
}
