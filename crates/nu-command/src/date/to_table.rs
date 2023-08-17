use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, Datelike, FixedOffset, Local, Timelike};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError::DatetimeParseError, ShellError::PipelineEmpty,
    Signature, Span, SpannedValue,
};
use nu_protocol::{ShellError, Type};

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
        let example_result_1 = || {
            let span = Span::test_data();
            let cols = vec![
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
                SpannedValue::Int { val: 2020, span },
                SpannedValue::Int { val: 4, span },
                SpannedValue::Int { val: 12, span },
                SpannedValue::Int { val: 22, span },
                SpannedValue::Int { val: 10, span },
                SpannedValue::Int { val: 57, span },
                SpannedValue::Int { val: 789, span },
                SpannedValue::String {
                    val: "+02:00".to_string(),
                    span,
                },
            ];
            Some(SpannedValue::List {
                vals: vec![SpannedValue::Record { cols, vals, span }],
                span,
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
                result: example_result_1(),
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

fn parse_date_into_table(
    date: Result<DateTime<FixedOffset>, SpannedValue>,
    head: Span,
) -> SpannedValue {
    let cols = vec![
        "year".into(),
        "month".into(),
        "day".into(),
        "hour".into(),
        "minute".into(),
        "second".into(),
        "nanosecond".into(),
        "timezone".into(),
    ];
    match date {
        Ok(x) => {
            let vals = vec![
                SpannedValue::int(x.year() as i64, head),
                SpannedValue::int(x.month() as i64, head),
                SpannedValue::int(x.day() as i64, head),
                SpannedValue::int(x.hour() as i64, head),
                SpannedValue::int(x.minute() as i64, head),
                SpannedValue::int(x.second() as i64, head),
                SpannedValue::int(x.nanosecond() as i64, head),
                SpannedValue::string(x.offset().to_string(), head),
            ];
            SpannedValue::List {
                vals: vec![SpannedValue::Record {
                    cols,
                    vals,
                    span: head,
                }],
                span: head,
            }
        }
        Err(e) => e,
    }
}

fn helper(val: SpannedValue, head: Span) -> SpannedValue {
    match val {
        SpannedValue::String {
            val,
            span: val_span,
        } => {
            let date = parse_date_from_string(&val, val_span);
            parse_date_into_table(date, head)
        }
        SpannedValue::Nothing { span: _ } => {
            let now = Local::now();
            let n = now.with_timezone(now.offset());
            parse_date_into_table(Ok(n), head)
        }
        SpannedValue::Date { val, span: _ } => parse_date_into_table(Ok(val), head),
        _ => SpannedValue::Error {
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
