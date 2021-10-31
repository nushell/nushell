use crate::date::utils::parse_date_from_string;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, Local};
use chrono_humanize::HumanTime;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, Value};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date humanize"
    }

    fn signature(&self) -> Signature {
        Signature::build("date humanize")
    }

    fn usage(&self) -> &str {
        "Print a 'humanized' format for the date, relative to now."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        input.map(move |value| helper(value, head), engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print a 'humanized' format for the date, relative to now.",
                example: "date humanize",
                result: Some(Value::String {
                    val: "now".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Print a 'humanized' format for the date, relative to now.",
                example: r#""2021-10-22 20:00:12 +01:00" | date humanize"#,
                result: {
                    let s = Local.ymd(2021, 10, 22).and_hms(20, 00, 12);
                    Some(Value::String {
                        val: HumanTime::from(s).to_string(),
                        span: Span::unknown(),
                    })
                },
            },
        ]
    }
}

fn helper(value: Value, head: Span) -> Value {
    match value {
        Value::Nothing { span: _ } => {
            let dt = Local::now();
            Value::String {
                val: humanize_date(dt.with_timezone(dt.offset())),
                span: head,
            }
        }
        Value::String { val, span: _ } => {
            let dt = parse_date_from_string(val);
            match dt {
                Ok(x) => Value::String {
                    val: humanize_date(x),
                    span: head,
                },
                Err(e) => e,
            }
        }
        Value::Date { val, span: _ } => Value::String {
            val: humanize_date(val),
            span: head,
        },
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                String::from("Date cannot be parsed / date format is not supported"),
                Span::unknown(),
            ),
        },
    }
}

fn humanize_date(dt: DateTime<FixedOffset>) -> String {
    HumanTime::from(dt).to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
