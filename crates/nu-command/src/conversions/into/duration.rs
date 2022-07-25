use nu_engine::CallExt;
use nu_parser::parse_duration_bytes;
use nu_protocol::{
    ast::{Call, CellPath, Expr},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Unit, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "column paths to convert to duration (for table input)",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to duration"
    }

    fn extra_usage(&self) -> &str {
        "into duration does not take leap years into account and every month is calculated with 30 days"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "time", "period"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_duration(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert string to duration in table",
                example: "echo [[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::Duration {
                                val: 1000 * 1000 * 1000,
                                span,
                            }],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::Duration {
                                val: 2 * 60 * 1000 * 1000 * 1000,
                                span,
                            }],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::Duration {
                                val: 3 * 60 * 60 * 1000 * 1000 * 1000,
                                span,
                            }],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::Duration {
                                val: 4 * 24 * 60 * 60 * 1000 * 1000 * 1000,
                                span,
                            }],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::Duration {
                                val: 5 * 7 * 24 * 60 * 60 * 1000 * 1000 * 1000,
                                span,
                            }],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert string to duration",
                example: "'7min' | into duration",
                result: Some(Value::Duration {
                    val: 7 * 60 * 1000 * 1000 * 1000,
                    span,
                }),
            },
        ]
    }
}

fn into_duration(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn string_to_duration(s: &str, span: Span, value_span: Span) -> Result<i64, ShellError> {
    if let Some(expression) = parse_duration_bytes(s.as_bytes(), span) {
        if let Expr::ValueWithUnit(value, unit) = expression.expr {
            if let Expr::Int(x) = value.expr {
                match unit.item {
                    Unit::Nanosecond => return Ok(x),
                    Unit::Microsecond => return Ok(x * 1000),
                    Unit::Millisecond => return Ok(x * 1000 * 1000),
                    Unit::Second => return Ok(x * 1000 * 1000 * 1000),
                    Unit::Minute => return Ok(x * 60 * 1000 * 1000 * 1000),
                    Unit::Hour => return Ok(x * 60 * 60 * 1000 * 1000 * 1000),
                    Unit::Day => return Ok(x * 24 * 60 * 60 * 1000 * 1000 * 1000),
                    Unit::Week => return Ok(x * 7 * 24 * 60 * 60 * 1000 * 1000 * 1000),
                    Unit::Month => return Ok(x * 30 * 24 * 60 * 60 * 1000 * 1000 * 1000), //30 days to a month
                    Unit::Year => return Ok(x * 365 * 24 * 60 * 60 * 1000 * 1000 * 1000), //365 days to a year
                    Unit::Decade => return Ok(x * 10 * 365 * 24 * 60 * 60 * 1000 * 1000 * 1000), //365 days to a year
                    _ => {}
                }
            }
        }
    }

    Err(ShellError::CantConvertWithValue(
        "duration".to_string(),
        "string".to_string(),
        s.to_string(),
        span,
        value_span,
        Some("supported units are ns, us, ms, sec, min, hr, day, and wk".to_string()),
    ))
}

fn action(input: &Value, span: Span) -> Value {
    match input {
        Value::Duration { .. } => input.clone(),
        Value::String {
            val,
            span: value_span,
        } => match string_to_duration(val, span, *value_span) {
            Ok(val) => Value::Duration { val, span },
            Err(error) => Value::Error { error },
        },
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into duration' does not support this input".into(),
                span,
            ),
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

    #[test]
    fn turns_ns_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("3ns");
        let expected = Value::Duration { val: 3, span };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_us_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("4us");
        let expected = Value::Duration {
            val: 4 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_ms_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("5ms");
        let expected = Value::Duration {
            val: 5 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_sec_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("1sec");
        let expected = Value::Duration {
            val: 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_min_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("7min");
        let expected = Value::Duration {
            val: 7 * 60 * 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_hr_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("42hr");
        let expected = Value::Duration {
            val: 42 * 60 * 60 * 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_day_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("123day");
        let expected = Value::Duration {
            val: 123 * 24 * 60 * 60 * 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_wk_to_duration() {
        let span = Span::test_data();
        let word = Value::test_string("3wk");
        let expected = Value::Duration {
            val: 3 * 7 * 24 * 60 * 60 * 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }
}
