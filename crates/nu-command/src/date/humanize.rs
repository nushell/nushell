use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, FixedOffset, Local};
use chrono_humanize::HumanTime;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date humanize"
    }

    fn signature(&self) -> Signature {
        Signature::build("date humanize")
            .input_output_types(vec![
                (Type::Date, Type::String),
                (Type::String, Type::String),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Print a 'humanized' format for the date, relative to now."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "relative",
            "now",
            "today",
            "tomorrow",
            "yesterday",
            "weekday",
            "weekday_name",
            "timezone",
        ]
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
        vec![Example {
            description: "Print a 'humanized' format for the date, relative to now.",
            example: r#""2021-10-22 20:00:12 +01:00" | date humanize"#,
            result: None,
        }]
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
        Value::String {
            val,
            span: val_span,
        } => {
            let dt = parse_date_from_string(&val, val_span);
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
            error: ShellError::DatetimeParseError(head),
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
