use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode};

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
        let mut signature = Render.signature();
        signature.name = "table".to_string();
        signature.category = Category::Deprecated;
        signature
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

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![DeprecationEntry {
            ty: DeprecationType::Command,
            report_mode: ReportMode::FirstUse,
            renamed: Some("render".to_string()),
            since: Some("0.106.0".to_string()),
            expected_removal: None,
            message: Some("change this to render".to_string()),
        }]
    }
}
