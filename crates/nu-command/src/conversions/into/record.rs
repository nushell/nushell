use crate::input_handler::{operate, CellPathOnlyArgs};
use chrono::{DateTime, Datelike, FixedOffset, Timelike};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into record"
    }

    fn signature(&self) -> Signature {
        Signature::build("into record")
            .input_output_types(vec![
                (Type::Int, Type::Record(vec![])),
                (Type::Number, Type::Record(vec![])),
                (Type::String, Type::Record(vec![])),
                (Type::Bool, Type::Record(vec![])),
                (Type::List(Box::new(Type::Any)), Type::Table(vec![])),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to record"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_record(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert value to boolean in table",
                example: "echo [[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(false, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(false, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert bool to boolean",
                example: "true | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert integer to boolean",
                example: "1 | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert decimal to boolean",
                example: "0.3 | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert decimal string to boolean",
                example: "'0.0' | into bool",
                result: Some(Value::boolean(false, span)),
            },
            Example {
                description: "convert string to boolean",
                example: "'true' | into bool",
                result: Some(Value::boolean(true, span)),
            },
        ]
    }
}

fn into_record(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    let ctrlc = engine_state.ctrlc.clone();

    operate(action, args, input, call.head, ctrlc)
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    eprintln!(
        "into_record::action: input = {:?}, type = {:?}, columns = {:?}",
        &input,
        &input.get_type(),
        &input.columns()
    );
    match input {
        // Value::Binary
        // Value::Block
        Value::Bool { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::boolean(*val, span)],
            span,
        },
        // Value::CellPath
        // Value::Closure
        // Value::CustomValue
        Value::Date { val, .. } => parse_date_into_record(Ok(*val), span),
        Value::Duration { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::Duration { val: *val, span }],
            span,
        },
        // Value::Error
        Value::Filesize { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::Filesize { val: *val, span }],
            span,
        },
        Value::Float { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::float(*val, span)],
            span,
        },
        // [ 1 2 3 ] | into record will come here
        Value::Int { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::int(*val, span)],
            span,
        },
        //TODO: This is making a table instead of a record
        // Value::List { vals, .. } => Value::Record {
        //     cols: vec!["value".into()],
        //     vals: vec![Value::List {
        //         vals: vals.to_vec(),
        //         span,
        //     }],
        //     span,
        // },
        // Value::List { vals, .. } => {
        //     let mut values = vec![];
        //     for (idx, val) in vals.iter().enumerate() {
        //         values.push(Value::Record {
        //             cols: vec![format!("col{}", idx)],
        //             vals: vec![*val],
        //             span,
        //         })
        //     }
        //     values
        // }
        Value::List { vals, .. } => {
            eprintln!("vals: {:?}", &vals);
            let mut cols = vec![];
            let mut values = vec![];
            for (idx, val) in vals.iter().enumerate() {
                cols.push(format!("col{}", idx));
                values.push(val.clone());
            }
            Value::Record {
                cols,
                vals: values,
                span,
            }
        }
        // Value::Nothing
        // Value::Range { val, .. } => Value::Record {
        //     cols: vec!["value".into()],
        //     vals: vec![Value::Range {
        //         val: val.into_range_iter(ctrlc.clone()),
        //         span,
        //     }],
        //     span,
        // },

        // Tables come here
        Value::Record { cols, vals, .. } => {
            eprint!("cols: {:?}, vals: {:?}", &cols, &vals);
            Value::Record {
                cols: cols.to_vec(),
                vals: vals.to_vec(),
                span,
            }
        }
        Value::String { val, .. } => Value::Record {
            cols: vec!["value".into()],
            vals: vec![Value::string(val, span)],
            span,
        },
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into bool' does not support this input".into(),
                span,
            ),
        },
    }
}

fn parse_date_into_record(date: Result<DateTime<FixedOffset>, Value>, span: Span) -> Value {
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
                    span,
                },
                Value::Int {
                    val: x.month() as i64,
                    span,
                },
                Value::Int {
                    val: x.day() as i64,
                    span,
                },
                Value::Int {
                    val: x.hour() as i64,
                    span,
                },
                Value::Int {
                    val: x.minute() as i64,
                    span,
                },
                Value::Int {
                    val: x.second() as i64,
                    span,
                },
                Value::String {
                    val: x.offset().to_string(),
                    span,
                },
            ];
            Value::Record { cols, vals, span }
        }
        Err(e) => e,
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
