use crate::date::utils::parse_date_from_string;
use chrono::{DateTime, Datelike, FixedOffset, Local, Timelike};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Type;
use nu_protocol::{
    Category, Example, PipelineData, ShellError::DatetimeParseError, Signature, Span, Value,
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
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
                "timezone".into(),
            ];
            let vals = vec![
                Value::Int { val: 2020, span },
                Value::Int { val: 4, span },
                Value::Int { val: 12, span },
                Value::Int { val: 22, span },
                Value::Int { val: 10, span },
                Value::Int { val: 57, span },
                Value::String {
                    val: "+02:00".to_string(),
                    span,
                },
            ];
            Some(Value::List {
                vals: vec![Value::Record { cols, vals, span }],
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
                example: "'2020-04-12 22:10:57 +0200' | date to-table",
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

fn parse_date_into_table(date: Result<DateTime<FixedOffset>, Value>, head: Span) -> Value {
    let cols = vec![
        "year".into(),
        "month".into(),
        "day".into(),
        "hour".into(),
        "minute".into(),
        "second".into(),
        "timezone".into(),
    ];
    match date {
        Ok(x) => {
            let vals = vec![
                Value::Int {
                    val: x.year() as i64,
                    span: head,
                },
                Value::Int {
                    val: x.month() as i64,
                    span: head,
                },
                Value::Int {
                    val: x.day() as i64,
                    span: head,
                },
                Value::Int {
                    val: x.hour() as i64,
                    span: head,
                },
                Value::Int {
                    val: x.minute() as i64,
                    span: head,
                },
                Value::Int {
                    val: x.second() as i64,
                    span: head,
                },
                Value::String {
                    val: x.offset().to_string(),
                    span: head,
                },
            ];
            Value::List {
                vals: vec![Value::Record {
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

fn helper(val: Value, head: Span) -> Value {
    match val {
        Value::String {
            val,
            span: val_span,
        } => {
            let date = parse_date_from_string(&val, val_span);
            parse_date_into_table(date, head)
        }
        Value::Nothing { span: _ } => {
            let now = Local::now();
            let n = now.with_timezone(now.offset());
            parse_date_into_table(Ok(n), head)
        }
        Value::Date { val, span: _ } => parse_date_into_table(Ok(val), head),
        _ => Value::Error {
            error: DatetimeParseError(head),
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
