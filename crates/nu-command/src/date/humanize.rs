use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, FixedOffset, Local};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DateHumanize;

impl Command for DateHumanize {
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

    fn description(&self) -> &str {
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| helper(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Print a 'humanized' format for the date, relative to now.",
            example: r#""2021-10-22 20:00:12 +01:00" | date humanize"#,
            result: None,
        }]
    }
}

fn helper(value: Value, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Nothing { .. } => {
            let dt = Local::now();
            Value::string(humanize_date(dt.with_timezone(dt.offset())), head)
        }
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, span);
            match dt {
                Ok(x) => Value::string(humanize_date(x), head),
                Err(e) => e,
            }
        }
        Value::Date { val, .. } => Value::string(humanize_date(val), head),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "date, string (that represents datetime), or nothing".into(),
                wrong_type: value.get_type().to_string(),
                dst_span: head,
                src_span: span,
            },
            head,
        ),
    }
}

fn humanize_date(dt: DateTime<FixedOffset>) -> String {
    nu_protocol::human_time_from_now(&dt).to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(DateHumanize {})
    }
}
