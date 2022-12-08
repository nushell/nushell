use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_protocol::ast::{CellPath, PathMember};
use nu_protocol::format_duration_as_timeperiod;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into cellpath"
    }

    fn signature(&self) -> Signature {
        Signature::build("into cellpath")
            .input_output_types(vec![(Type::String, Type::CellPath)])
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to a cellpath."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "cellpath"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_cellpath(engine_state, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert from one row table to record",
                example: "echo [[value]; [false]] | into record",
                result: Some(Value::Record {
                    cols: vec!["value".to_string()],
                    vals: vec![Value::boolean(false, span)],
                    span,
                }),
            },
            Example {
                description: "Convert from list to record",
                example: "[1 2 3] | into record",
                result: Some(Value::Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 2, span },
                        Value::Int { val: 3, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert from range to record",
                example: "0..2 | into record",
                result: Some(Value::Record {
                    cols: vec!["0".to_string(), "1".to_string(), "2".to_string()],
                    vals: vec![
                        Value::Int { val: 0, span },
                        Value::Int { val: 1, span },
                        Value::Int { val: 2, span },
                    ],
                    span,
                }),
            },
            Example {
                description: "convert duration to record",
                example: "-500day | into record",
                result: Some(Value::Record {
                    cols: vec![
                        "year".into(),
                        "month".into(),
                        "week".into(),
                        "day".into(),
                        "sign".into(),
                    ],
                    vals: vec![
                        Value::Int { val: 1, span },
                        Value::Int { val: 4, span },
                        Value::Int { val: 2, span },
                        Value::Int { val: 1, span },
                        Value::String {
                            val: "-".into(),
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "convert record to record",
                example: "{a: 1, b: 2} | into record",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::Int { val: 1, span }, Value::Int { val: 2, span }],
                    span,
                }),
            },
            Example {
                description: "convert date to record",
                example: "2020-04-12T22:10:57+02:00 | into record",
                result: Some(Value::Record {
                    cols: vec![
                        "year".into(),
                        "month".into(),
                        "day".into(),
                        "hour".into(),
                        "minute".into(),
                        "second".into(),
                        "timezone".into(),
                    ],
                    vals: vec![
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
                    ],
                    span,
                }),
            },
        ]
    }
}

fn into_cellpath(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let input = input.into_value(call.head);
    let res = match input {
        Value::String { val, span } => parse_string_into_cellapth(val, span),
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into record' does not support this input".into(),
                other.span().unwrap_or(call.head),
            ),
        },
    };
    Ok(res.into_pipeline_data())
}

fn parse_string_into_cellapth(val: String, span: Span) -> Value {
    let parts = val.split('.').collect::<Vec<&str>>();
    let mut cellpath: Vec<PathMember> = vec![];
    for part in parts {
        cellpath.push(PathMember::String {
            val: part.to_string(),
            span: Span::new(0, 0),
        })
    }
    let res = Value::CellPath {
        val: CellPath { members: cellpath },
        span,
    };
    // dbg!(&res);
    res
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
