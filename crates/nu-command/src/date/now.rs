use chrono::Local;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Type, Value};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date now"
    }

    fn signature(&self) -> Signature {
        Signature::build("date now")
            .input_output_types(vec![(Type::Nothing, Type::Date)])
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Get the current date."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["present", "current-time"]
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
            Example {
                description: "Get the time duration from 2019-04-30 to now",
                example: r#"(date now) - 2019-05-01"#,
                result: None,
            },
            Example {
                description: "Get the time duration since a more accurate time",
                example: r#"(date now) - 2019-05-01T04:12:05.20+08:00"#,
                result: None,
            },
            Example {
                description: "Get current time in full RFC3339 format with timezone",
                example: r#"date now | debug"#,
                result: None,
            },
        ]
    }
}
