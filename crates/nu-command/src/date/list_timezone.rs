use chrono_tz::TZ_VARIANTS;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoInterruptiblePipelineData, PipelineData, Signature, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date list_timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date list_timezone").category(Category::Date)
    }

    fn usage(&self) -> &str {
        "List supported time zones."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;

        Ok(TZ_VARIANTS
            .iter()
            .map(move |x| {
                let cols = vec!["timezone".into()];
                let vals = vec![Value::String {
                    val: x.name().to_string(),
                    span,
                }];
                Value::Record { cols, vals, span }
            })
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}
