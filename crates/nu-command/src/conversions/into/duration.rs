use nu_engine::CallExt;
use nu_parser::parse_duration_bytes;
use nu_protocol::{
    ast::{Call, CellPath, Expr},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Unit,
    Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .named(
                "convert",
                SyntaxShape::String,
                "convert duration into another duration",
                Some('c'),
            )
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
            Example {
                description: "Convert string to a named duration",
                example: "'7min' | into duration --convert sec",
                result: Some(Value::String {
                    val: "420 sec".to_string(),
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
    let convert_to_unit: Option<Spanned<String>> = call.get_flag(engine_state, stack, "convert")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &convert_to_unit, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let d = convert_to_unit.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &d, head)),
                    );
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

fn convert_str_from_unit_to_unit(
    val: i64,
    from_unit: &str,
    to_unit: &str,
    span: Span,
    value_span: Span,
) -> Result<i64, ShellError> {
    match (from_unit, to_unit) {
        ("ns", "ns") => Ok(val),
        ("ns", "us") => Ok(val / 1000),
        ("ns", "ms") => Ok(val / 1000 / 1000),
        ("ns", "sec") => Ok(val / 1000 / 1000 / 1000),
        ("ns", "min") => Ok(val / 1000 / 1000 / 1000 / 60),
        ("ns", "hr") => Ok(val / 1000 / 1000 / 1000 / 60 / 60),
        ("ns", "day") => Ok(val / 1000 / 1000 / 1000 / 60 / 60 / 24),
        ("ns", "wk") => Ok(val / 1000 / 1000 / 1000 / 60 / 60 / 24 / 7),
        ("ns", "month") => Ok(val / 1000 / 1000 / 1000 / 60 / 60 / 24 / 30),
        ("ns", "yr") => Ok(val / 1000 / 1000 / 1000 / 60 / 60 / 24 / 365),
        ("ns", "dec") => Ok(val / 10 / 1000 / 1000 / 1000 / 60 / 60 / 24 / 365),

        ("us", "ns") => Ok(val * 1000),
        ("us", "us") => Ok(val),
        ("us", "ms") => Ok(val / 1000),
        ("us", "sec") => Ok(val / 1000 / 1000),
        ("us", "min") => Ok(val / 1000 / 1000 / 60),
        ("us", "hr") => Ok(val / 1000 / 1000 / 60 / 60),
        ("us", "day") => Ok(val / 1000 / 1000 / 60 / 60 / 24),
        ("us", "wk") => Ok(val / 1000 / 1000 / 60 / 60 / 24 / 7),
        ("us", "month") => Ok(val / 1000 / 1000 / 60 / 60 / 24 / 30),
        ("us", "yr") => Ok(val / 1000 / 1000 / 60 / 60 / 24 / 365),
        ("us", "dec") => Ok(val / 10 / 1000 / 1000 / 60 / 60 / 24 / 365),

        ("ms", "ns") => Ok(val * 1000 * 1000),
        ("ms", "us") => Ok(val * 1000),
        ("ms", "ms") => Ok(val),
        ("ms", "sec") => Ok(val / 1000),
        ("ms", "min") => Ok(val / 1000 / 60),
        ("ms", "hr") => Ok(val / 1000 / 60 / 60),
        ("ms", "day") => Ok(val / 1000 / 60 / 60 / 24),
        ("ms", "wk") => Ok(val / 1000 / 60 / 60 / 24 / 7),
        ("ms", "month") => Ok(val / 1000 / 60 / 60 / 24 / 30),
        ("ms", "yr") => Ok(val / 1000 / 60 / 60 / 24 / 365),
        ("ms", "dec") => Ok(val / 10 / 1000 / 60 / 60 / 24 / 365),

        ("sec", "ns") => Ok(val * 1000 * 1000 * 1000),
        ("sec", "us") => Ok(val * 1000 * 1000),
        ("sec", "ms") => Ok(val * 1000),
        ("sec", "sec") => Ok(val),
        ("sec", "min") => Ok(val / 60),
        ("sec", "hr") => Ok(val / 60 / 60),
        ("sec", "day") => Ok(val / 60 / 60 / 24),
        ("sec", "wk") => Ok(val / 60 / 60 / 24 / 7),
        ("sec", "month") => Ok(val / 60 / 60 / 24 / 30),
        ("sec", "yr") => Ok(val / 60 / 60 / 24 / 365),
        ("sec", "dec") => Ok(val / 10 / 60 / 60 / 24 / 365),

        ("min", "ns") => Ok(val * 1000 * 1000 * 1000 * 60),
        ("min", "us") => Ok(val * 1000 * 1000 * 60),
        ("min", "ms") => Ok(val * 1000 * 60),
        ("min", "sec") => Ok(val * 60),
        ("min", "min") => Ok(val),
        ("min", "hr") => Ok(val / 60),
        ("min", "day") => Ok(val / 60 / 24),
        ("min", "wk") => Ok(val / 60 / 24 / 7),
        ("min", "month") => Ok(val / 60 / 24 / 30),
        ("min", "yr") => Ok(val / 60 / 24 / 365),
        ("min", "dec") => Ok(val / 10 / 60 / 24 / 365),

        ("hr", "ns") => Ok(val * 1000 * 1000 * 1000 * 60 * 60),
        ("hr", "us") => Ok(val * 1000 * 1000 * 60 * 60),
        ("hr", "ms") => Ok(val * 1000 * 60 * 60),
        ("hr", "sec") => Ok(val * 60 * 60),
        ("hr", "min") => Ok(val * 60),
        ("hr", "hr") => Ok(val),
        ("hr", "day") => Ok(val / 24),
        ("hr", "wk") => Ok(val / 24 / 7),
        ("hr", "month") => Ok(val / 24 / 30),
        ("hr", "yr") => Ok(val / 24 / 365),
        ("hr", "dec") => Ok(val / 10 / 24 / 365),

        ("day", "ns") => Ok(val * 1000 * 1000 * 1000 * 60 * 60 * 24),
        ("day", "us") => Ok(val * 1000 * 1000 * 60 * 60 * 24),
        ("day", "ms") => Ok(val * 1000 * 60 * 60 * 24),
        ("day", "sec") => Ok(val * 60 * 60 * 24),
        ("day", "min") => Ok(val * 60 * 24),
        ("day", "hr") => Ok(val * 24),
        ("day", "day") => Ok(val),
        ("day", "wk") => Ok(val / 7),
        ("day", "month") => Ok(val / 30),
        ("day", "yr") => Ok(val / 365),
        ("day", "dec") => Ok(val / 10 / 365),

        ("wk", "ns") => Ok(val * 1000 * 1000 * 1000 * 60 * 60 * 24 * 7),
        ("wk", "us") => Ok(val * 1000 * 1000 * 60 * 60 * 24 * 7),
        ("wk", "ms") => Ok(val * 1000 * 60 * 60 * 24 * 7),
        ("wk", "sec") => Ok(val * 60 * 60 * 24 * 7),
        ("wk", "min") => Ok(val * 60 * 24 * 7),
        ("wk", "hr") => Ok(val * 24 * 7),
        ("wk", "day") => Ok(val * 7),
        ("wk", "wk") => Ok(val),
        ("wk", "month") => Ok(val / 4),
        ("wk", "yr") => Ok(val / 52),
        ("wk", "dec") => Ok(val / 10 / 52),

        ("month", "ns") => Ok(val * 1000 * 1000 * 1000 * 60 * 60 * 24 * 30),
        ("month", "us") => Ok(val * 1000 * 1000 * 60 * 60 * 24 * 30),
        ("month", "ms") => Ok(val * 1000 * 60 * 60 * 24 * 30),
        ("month", "sec") => Ok(val * 60 * 60 * 24 * 30),
        ("month", "min") => Ok(val * 60 * 24 * 30),
        ("month", "hr") => Ok(val * 24 * 30),
        ("month", "day") => Ok(val * 30),
        ("month", "wk") => Ok(val * 4),
        ("month", "month") => Ok(val),
        ("month", "yr") => Ok(val / 12),
        ("month", "dec") => Ok(val / 10 / 12),

        ("yr", "ns") => Ok(val * 1000 * 1000 * 1000 * 60 * 60 * 24 * 365),
        ("yr", "us") => Ok(val * 1000 * 1000 * 60 * 60 * 24 * 365),
        ("yr", "ms") => Ok(val * 1000 * 60 * 60 * 24 * 365),
        ("yr", "sec") => Ok(val * 60 * 60 * 24 * 365),
        ("yr", "min") => Ok(val * 60 * 24 * 365),
        ("yr", "hr") => Ok(val * 24 * 365),
        ("yr", "day") => Ok(val * 365),
        ("yr", "wk") => Ok(val * 52),
        ("yr", "month") => Ok(val * 12),
        ("yr", "yr") => Ok(val),
        ("yr", "dec") => Ok(val / 10),

        _ => Err(ShellError::CantConvertWithValue(
            "string duration".to_string(),
            "string duration".to_string(),
            to_unit.to_string(),
            span,
            value_span,
            Some(
                "supported units are ns, us, ms, sec, min, hr, day, wk, month, yr and dec"
                    .to_string(),
            ),
        )),
    }
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
        Some(
            "supported units are ns, us, ms, sec, min, hr, day, wk, month, yr and dec".to_string(),
        ),
    ))
}

fn string_to_unit_duration(
    s: &str,
    span: Span,
    value_span: Span,
) -> Result<(&str, i64), ShellError> {
    if let Some(expression) = parse_duration_bytes(s.as_bytes(), span) {
        if let Expr::ValueWithUnit(value, unit) = expression.expr {
            if let Expr::Int(x) = value.expr {
                match unit.item {
                    Unit::Nanosecond => return Ok(("ns", x)),
                    Unit::Microsecond => return Ok(("us", x)),
                    Unit::Millisecond => return Ok(("ms", x)),
                    Unit::Second => return Ok(("sec", x)),
                    Unit::Minute => return Ok(("min", x)),
                    Unit::Hour => return Ok(("hr", x)),
                    Unit::Day => return Ok(("day", x)),
                    Unit::Week => return Ok(("wk", x)),

                    _ => return Ok(("ns", 0)),
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
        Some(
            "supported units are ns, us, ms, sec, min, hr, day, wk, month, yr and dec".to_string(),
        ),
    ))
}

fn action(input: &Value, convert_to_unit: &Option<Spanned<String>>, span: Span) -> Value {
    match input {
        Value::Duration {
            val: _val_num,
            span: _value_span,
        } => {
            if let Some(_to_unit) = convert_to_unit {
                Value::Error {
                    error: ShellError::UnsupportedInput(
                        "Cannot convert from a Value::Duration right now. Try making it a string."
                            .into(),
                        span,
                    ),
                }
            } else {
                input.clone()
            }
        }
        Value::String {
            val,
            span: value_span,
        } => {
            if let Some(to_unit) = convert_to_unit {
                if let Ok(dur) = string_to_unit_duration(val, span, *value_span) {
                    let from_unit = dur.0;
                    let duration = dur.1;
                    match convert_str_from_unit_to_unit(
                        duration,
                        from_unit,
                        &to_unit.item,
                        span,
                        *value_span,
                    ) {
                        Ok(d) => Value::String {
                            val: format!("{} {}", d, &to_unit.item),
                            span: *value_span,
                        },
                        Err(e) => Value::Error { error: e },
                    }
                } else {
                    Value::Error {
                        error: ShellError::UnsupportedInput(
                            "'into duration' does not support this string input".into(),
                            span,
                        ),
                    }
                }
            } else {
                match string_to_duration(val, span, *value_span) {
                    Ok(val) => Value::Duration { val, span },
                    Err(error) => Value::Error { error },
                }
            }
        }
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
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
        let convert_duration = None;

        let actual = action(&word, &convert_duration, span);
        assert_eq!(actual, expected);
    }
}
