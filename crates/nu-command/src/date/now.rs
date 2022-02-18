use chrono::Local;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Value};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date now"
    }

    fn signature(&self) -> Signature {
        Signature::build("date now").category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Get the current date."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let dt = Local::now();
        Ok(Value::Date {
            val: dt.with_timezone(dt.offset()),
            span: head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the current date and display it in a given format string.",
                example: r#"date now | date format "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
        ]
    }
}
