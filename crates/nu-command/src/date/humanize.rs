use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, FixedOffset, Local};
use chrono_humanize::HumanTime;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
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

fn helper(value: SpannedValue, head: Span) -> SpannedValue {
    match value {
        SpannedValue::Nothing { span: _ } => {
            let dt = Local::now();
            SpannedValue::String {
                val: humanize_date(dt.with_timezone(dt.offset())),
                span: head,
            }
        }
        SpannedValue::String {
            val,
            span: val_span,
        } => {
            let dt = parse_date_from_string(&val, val_span);
            match dt {
                Ok(x) => SpannedValue::String {
                    val: humanize_date(x),
                    span: head,
                },
                Err(e) => e,
            }
        }
        SpannedValue::Date { val, span: _ } => SpannedValue::String {
            val: humanize_date(val),
            span: head,
        },
        _ => SpannedValue::Error {
            error: Box::new(ShellError::DatetimeParseError(value.debug_value(), head)),
            span: head,
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
