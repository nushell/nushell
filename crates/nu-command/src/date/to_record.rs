use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, Datelike, FixedOffset, Local, Timelike};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{record, ShellError, Type};
use nu_protocol::{
    Category, Example, PipelineData, ShellError::DatetimeParseError, ShellError::PipelineEmpty,
    Signature, Span, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date to-record"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-record")
            .input_output_types(vec![
                (Type::Date, Type::Record(vec![])),
                (Type::String, Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Convert the date into a record."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["structured", "table"]
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
            return Err(PipelineEmpty { dst_span: head });
        }
        input.map(move |value| helper(value, head), engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        let example_result_1 = {
            let cols: Vec<String> = vec![
                "year".into(),
                "month".into(),
                "day".into(),
                "hour".into(),
                "minute".into(),
                "second".into(),
                "nanosecond".into(),
                "timezone".into(),
            ];
            let vals = vec![
                Value::test_int(2020),
                Value::test_int(4),
                Value::test_int(12),
                Value::test_int(22),
                Value::test_int(10),
                Value::test_int(57),
                Value::test_int(123_000_000),
                Value::test_string("+02:00"),
            ];
            Some(Value::test_record_from_parts(cols, vals))
        };

        vec![
            Example {
                description: "Convert the current date into a record.",
                example: "date to-record",
                result: None,
            },
            Example {
                description: "Convert the current date into a record.",
                example: "date now | date to-record",
                result: None,
            },
            Example {
                description: "Convert a date string into a record.",
                example: "'2020-04-12T22:10:57.123+02:00' | date to-record",
                result: example_result_1,
            },
            // TODO: This should work but does not; see https://github.com/nushell/nushell/issues/7032
            // Example {
            //     description: "Convert a date into a record.",
            //     example: "'2020-04-12 22:10:57 +0200' | into datetime | date to-record",
            //     result: example_result_1(),
            // },
        ]
    }
}

fn parse_date_into_table(date: DateTime<FixedOffset>, head: Span) -> Value {
    Value::record(
        record! {
            year => Value::int(date.year() as i64, head),
            month => Value::int(date.month() as i64, head),
            day => Value::int(date.day() as i64, head),
            hour => Value::int(date.hour() as i64, head),
            minute => Value::int(date.minute() as i64, head),
            second => Value::int(date.second() as i64, head),
            nanosecond => Value::int(date.nanosecond() as i64, head),
            timezone => Value::string(date.offset().to_string(), head),
        },
        head,
    )
}

fn helper(val: Value, head: Span) -> Value {
    match val {
        Value::String {
            val,
            span: val_span,
        } => match parse_date_from_string(&val, val_span) {
            Ok(date) => parse_date_into_table(date, head),
            Err(e) => e,
        },
        Value::Nothing { span: _ } => {
            let now = Local::now();
            let n = now.with_timezone(now.offset());
            parse_date_into_table(n, head)
        }
        Value::Date { val, span: _ } => parse_date_into_table(val, head),
        _ => Value::Error {
            error: Box::new(DatetimeParseError(val.debug_value(), head)),
        },
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
