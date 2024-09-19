use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Commandline;

impl Command for Commandline {
    fn name(&self) -> &str {
        "commandline"
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "View the current command line input buffer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let repl = engine_state.repl_state.lock().expect("repl state mutex");
        Ok(Value::string(repl.buffer.clone(), call.head).into_pipeline_data())
    }
}
