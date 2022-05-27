use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, Signature, Value,
};

#[derive(Clone)]
pub struct Keybindings;

impl Command for Keybindings {
    fn name(&self) -> &str {
        "keybindings"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "Keybindings related commands"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shortcut", "hotkey"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &Keybindings.signature(),
                &Keybindings.examples(),
                engine_state,
                stack,
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
