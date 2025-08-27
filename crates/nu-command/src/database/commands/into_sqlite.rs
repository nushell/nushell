use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode};

#[derive(Clone)]
pub struct IntoSqliteDb(pub(crate) super::ToSqliteDb);

impl Command for IntoSqliteDb {
    fn deprecation_info(&self) -> Vec<nu_protocol::DeprecationEntry> {
        vec![DeprecationEntry {
            ty: DeprecationType::Command,
            report_mode: ReportMode::FirstUse,
            since: Some("0.107.0".into()),
            expected_removal: None,
            help: Some(
                "to align the behavior of this command, it is now called `to sqlite`".into(),
            ),
        }]
    }

    fn name(&self) -> &str {
        "into sqlite"
    }

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().into(),
            ..self.0.signature()
        }
    }

    fn description(&self) -> &str {
        self.0.description()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> std::result::Result<PipelineData, ShellError> {
        self.0.run(engine_state, stack, call, input)
    }

    fn extra_description(&self) -> &str {
        self.0.extra_description()
    }

    fn examples(&self) -> Vec<Example> {
        self.0.examples()
    }
}
