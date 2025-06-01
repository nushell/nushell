use nu_engine::command_prelude::*;

use super::Render;

#[derive(Clone)]
pub struct Table;

impl Command for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn description(&self) -> &str {
        "Deprecated command, use the render command instead."
    }

    fn extra_description(&self) -> &str {
        "If the table contains a column called 'index', this column is used as the table index instead of the usual continuous index."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render"]
    }

    fn signature(&self) -> Signature {
        Render.signature().category(Category::Deprecated)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Render.run(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        Render.examples()
    }
}
