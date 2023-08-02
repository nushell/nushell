use nu_engine::CallExt;
use nu_parser::{parse_unit_value, DURATION_UNIT_GROUPS};
use nu_protocol::{
    ast::{Call, CellPath, Expr},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Unit, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .input_output_types(vec![
                (Type::String, Type::Duration),
                (Type::Duration, Type::Duration),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to duration."
    }

    fn extra_usage(&self) -> &str {
        "This command does not take leap years into account, and every month is assumed to have 30 days."
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
    ) -> Result<PipelineData, ShellError> {
        into_duration(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert string to duration in table",
                example:
                    "[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value",
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
            Example {
                description: "Convert duration to duration",
                example: "420sec | into duration",
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
    let span = match input.span() {
        Some(t) => t,
        None => call.head,
    };
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, span)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, span)));
                    if let Err(error) = r {
                        return Value::Error {
                            error: Box::new(error),
                        };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn string_to_duration(s: &str, span: Span, value_span: Span) -> Result<i64, ShellError> {
    if let Some(Ok(expression)) = parse_unit_value(
        s.as_bytes(),
        span,
        DURATION_UNIT_GROUPS,
        Type::Duration,
        |x| x,
    ) {
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
                    _ => {}
                }
            }
        }
    }

    Err(ShellError::CantConvertToDuration {
        details: s.to_string(),
        dst_span: span,
        src_span: value_span,
        help: Some(
            "supported units are ns, us/µs, ms, sec, min, hr, day, wk, month, yr, and dec"
                .to_string(),
        ),
    })
}

fn action(input: &Value, span: Span) -> Value {
    match input {
        Value::Duration { .. } => input.clone(),
        Value::String {
            val,
            span: value_span,
        } => match string_to_duration(val, span, *value_span) {
            Ok(val) => Value::Duration { val, span },
            Err(error) => Value::Error {
                error: Box::new(error),
            },
        },
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string or duration".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.expect_span(),
            }),
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
        let span = Span::new(0, 2);
        let word = Value::test_string("3ns");
        let expected = Value::Duration { val: 3, span };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_us_to_duration() {
        let span = Span::new(0, 2);
        let word = Value::test_string("4us");
        let expected = Value::Duration {
            val: 4 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_micro_sign_s_to_duration() {
        let span = Span::new(0, 2);
        let word = Value::test_string("4\u{00B5}s");
        let expected = Value::Duration {
            val: 4 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_mu_s_to_duration() {
        let span = Span::new(0, 2);
        let word = Value::test_string("4\u{03BC}s");
        let expected = Value::Duration {
            val: 4 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_ms_to_duration() {
        let span = Span::new(0, 2);
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
        let span = Span::new(0, 3);
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
        let span = Span::new(0, 3);
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
        let span = Span::new(0, 3);
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
        let span = Span::new(0, 5);
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
        let span = Span::new(0, 2);
        let word = Value::test_string("3wk");
        let expected = Value::Duration {
            val: 3 * 7 * 24 * 60 * 60 * 1000 * 1000 * 1000,
            span,
        };

        let actual = action(&word, span);
        assert_eq!(actual, expected);
    }
}
