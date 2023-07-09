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
        "date to-table"
    }

    fn signature(&self) -> Signature {
        Signature::build("date to-table")
            .input_output_types(vec![
                (Type::Date, Type::Table(vec![])),
                (Type::String, Type::Table(vec![])),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
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
                Value::test_int(789),
                Value::test_string("+02:00"),
            ];
            Some(Value::List {
                vals: vec![Value::test_record_from_parts(cols, vals)],
                span: Span::test_data(),
            })
        };

        vec![
            Example {
                description: "Convert the current date into a table.",
                example: "date to-table",
                result: None,
            },
            Example {
                description: "Convert the date into a table.",
                example: "date now | date to-table",
                result: None,
            },
            Example {
                description: "Convert a given date into a table.",
                //todo: resolve https://github.com/bspeice/dtparse/issues/40, which truncates nanosec bits
                // for now, change the example to use date literal rather than string conversion, as workaround
                example: "2020-04-12T22:10:57.000000789+02:00 | date to-table",
                result: example_result_1,
            },
            // TODO: This should work but does not; see https://github.com/nushell/nushell/issues/7032
            // Example {
            //     description: "Convert a given date into a table.",
            //     example: "'2020-04-12 22:10:57 +0200' | into datetime | date to-table",
            //     result: example_result_1(),
            // },
        ]
    }
}

fn parse_date_into_table(date: DateTime<FixedOffset>, head: Span) -> Value {
    let record = record! {
        year => Value::int(date.year() as i64, head),
        month => Value::int(date.month() as i64, head),
        day => Value::int(date.day() as i64, head),
        hour => Value::int(date.hour() as i64, head),
        minute => Value::int(date.minute() as i64, head),
        second => Value::int(date.second() as i64, head),
        nanoecond => Value::int(date.nanosecond() as i64, head),
        timezone => Value::string(date.offset().to_string(), head),
    };

    Value::list(vec![Value::record(record, head)], head)
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
