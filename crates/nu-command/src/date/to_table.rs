use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, Datelike, FixedOffset, Local, Timelike};
use nu_engine::command_prelude::*;
use nu_protocol::{report_parse_warning, ParseWarning};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date to-table"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-table")
            .input_output_types(vec![
                (Type::Date, Type::table()),
                (Type::String, Type::table()),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .category(Category::Deprecated)
    }

    fn description(&self) -> &str {
        "Convert the date into a structured table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["structured"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        report_parse_warning(
            &StateWorkingSet::new(engine_state),
            &ParseWarning::DeprecatedWarning {
                old_command: "date to-table".into(),
                new_suggestion: "see `into record` command examples".into(),
                span: head,
                url: "`help into record`".into(),
            },
        );

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| helper(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert the current date into a table.",
                example: "date now | date to-table",
                result: None,
            },
            Example {
                description: "Convert a given date into a table.",
                example: "2020-04-12T22:10:57.000000789+02:00 | date to-table",
                result: Some(Value::test_list(vec![Value::test_record(record!(
                    "year" =>       Value::test_int(2020),
                    "month" =>      Value::test_int(4),
                    "day" =>        Value::test_int(12),
                    "hour" =>       Value::test_int(22),
                    "minute" =>     Value::test_int(10),
                    "second" =>     Value::test_int(57),
                    "nanosecond" => Value::test_int(789),
                    "timezone" =>   Value::test_string("+02:00".to_string()),
                ))])),
            },
            Example {
                description: "Convert a given date into a table.",
                example: "'2020-04-12 22:10:57 +0200' | into datetime | date to-table",
                result: Some(Value::test_list(vec![Value::test_record(record!(
                    "year" =>       Value::test_int(2020),
                    "month" =>      Value::test_int(4),
                    "day" =>        Value::test_int(12),
                    "hour" =>       Value::test_int(22),
                    "minute" =>     Value::test_int(10),
                    "second" =>     Value::test_int(57),
                    "nanosecond" => Value::test_int(0),
                    "timezone" =>   Value::test_string("+02:00".to_string()),
                ))])),
            },
        ]
    }
}

fn parse_date_into_table(date: DateTime<FixedOffset>, head: Span) -> Value {
    let record = record! {
        "year" => Value::int(date.year() as i64, head),
        "month" => Value::int(date.month() as i64, head),
        "day" => Value::int(date.day() as i64, head),
        "hour" => Value::int(date.hour() as i64, head),
        "minute" => Value::int(date.minute() as i64, head),
        "second" => Value::int(date.second() as i64, head),
        "nanosecond" => Value::int(date.nanosecond() as i64, head),
        "timezone" => Value::string(date.offset().to_string(), head),
    };

    Value::list(vec![Value::record(record, head)], head)
}

fn helper(val: Value, head: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::String { val, .. } => match parse_date_from_string(&val, val_span) {
            Ok(date) => parse_date_into_table(date, head),
            Err(e) => e,
        },
        Value::Nothing { .. } => {
            let now = Local::now();
            let n = now.with_timezone(now.offset());
            parse_date_into_table(n, head)
        }
        Value::Date { val, .. } => parse_date_into_table(val, head),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "date, string (that represents datetime), or nothing".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: head,
                src_span: val_span,
            },
            head,
        ),
    }
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
