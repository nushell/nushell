use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct ClipCommand;

impl Command for ClipCommand {
    fn name(&self) -> &str {
        "clip"
    }

    fn signature(&self) -> Signature {
        Signature::build("clip")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Commands for managing the clipboard."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "paste", "clipboard"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }
}
