use chrono::Local;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DateNow;

impl Command for DateNow {
    fn name(&self) -> &str {
        "date now"
    }

    fn signature(&self) -> Signature {
        Signature::build("date now")
            .input_output_types(vec![(Type::Nothing, Type::Date)])
            .category(Category::Date)
    }

    fn description(&self) -> &str {
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let dt = Local::now();
        Ok(Value::date(dt.with_timezone(dt.offset()), head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the current date and format it in a given format string.",
                example: r#"date now | format date "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Get the current date and format it according to the RFC 3339 standard.",
                example: r#"date now | format date "%+""#,
                result: None,
            },
            Example {
                description: "Get the time duration since 2019-04-30.",
                example: r#"(date now) - 2019-05-01"#,
                result: None,
            },
            Example {
                description: "Get the time duration since a more specific time.",
                example: r#"(date now) - 2019-05-01T04:12:05.20+08:00"#,
                result: None,
            },
            Example {
                description: "Get current time and format it in the debug format (RFC 2822 with timezone)",
                example: r#"date now | debug"#,
                result: None,
            },
        ]
    }
}
