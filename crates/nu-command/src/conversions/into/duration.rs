use nu_engine::CallExt;
use nu_parser::parse_duration_bytes;
use nu_protocol::{
    ast::{Call, CellPath, Expr},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Unit,
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
            .input_output_types(vec![
                (Type::String, Type::Duration),
                (Type::Duration, Type::Duration),
                // TODO: --convert option should be implemented as `format duration`
                (Type::String, Type::String),
                (Type::Duration, Type::String),
            ])
            .named(
                "convert",
                SyntaxShape::String,
                "convert duration into another duration",
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to duration"
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
                description: "Convert string to the requested duration as a string",
                example: "'7min' | into duration --convert sec",
                result: Some(Value::String {
                    val: "420 sec".to_string(),
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
            Example {
                description: "Convert duration to the requested duration as a string",
                example: "420sec | into duration --convert ms",
                result: Some(Value::String {
                    val: "420000 ms".to_string(),
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
    let config = engine_state.get_config();
    let float_precision = config.float_precision as usize;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &convert_to_unit, float_precision, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let d = convert_to_unit.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &d, float_precision, head)),
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
) -> Result<f64, ShellError> {
    match (from_unit, to_unit) {
        ("ns", "ns") => Ok(val as f64),
        ("ns", "us") => Ok(val as f64 / 1000.0),
        ("ns", "ms") => Ok(val as f64 / 1000.0 / 1000.0),
        ("ns", "sec") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0),
        ("ns", "min") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0),
        ("ns", "hr") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0),
        ("ns", "day") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0),
        ("ns", "wk") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 7.0),
        ("ns", "month") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 30.0),
        ("ns", "yr") => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),
        ("ns", "dec") => {
            Ok(val as f64 / 10.0 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0)
        }

        ("us", "ns") => Ok(val as f64 * 1000.0),
        ("us", "us") => Ok(val as f64),
        ("us", "ms") => Ok(val as f64 / 1000.0),
        ("us", "sec") => Ok(val as f64 / 1000.0 / 1000.0),
        ("us", "min") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0),
        ("us", "hr") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0 / 60.0),
        ("us", "day") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0),
        ("us", "wk") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 7.0),
        ("us", "month") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 30.0),
        ("us", "yr") => Ok(val as f64 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),
        ("us", "dec") => Ok(val as f64 / 10.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),

        ("ms", "ns") => Ok(val as f64 * 1000.0 * 1000.0),
        ("ms", "us") => Ok(val as f64 * 1000.0),
        ("ms", "ms") => Ok(val as f64),
        ("ms", "sec") => Ok(val as f64 / 1000.0),
        ("ms", "min") => Ok(val as f64 / 1000.0 / 60.0),
        ("ms", "hr") => Ok(val as f64 / 1000.0 / 60.0 / 60.0),
        ("ms", "day") => Ok(val as f64 / 1000.0 / 60.0 / 60.0 / 24.0),
        ("ms", "wk") => Ok(val as f64 / 1000.0 / 60.0 / 60.0 / 24.0 / 7.0),
        ("ms", "month") => Ok(val as f64 / 1000.0 / 60.0 / 60.0 / 24.0 / 30.0),
        ("ms", "yr") => Ok(val as f64 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),
        ("ms", "dec") => Ok(val as f64 / 10.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),

        ("sec", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0),
        ("sec", "us") => Ok(val as f64 * 1000.0 * 1000.0),
        ("sec", "ms") => Ok(val as f64 * 1000.0),
        ("sec", "sec") => Ok(val as f64),
        ("sec", "min") => Ok(val as f64 / 60.0),
        ("sec", "hr") => Ok(val as f64 / 60.0 / 60.0),
        ("sec", "day") => Ok(val as f64 / 60.0 / 60.0 / 24.0),
        ("sec", "wk") => Ok(val as f64 / 60.0 / 60.0 / 24.0 / 7.0),
        ("sec", "month") => Ok(val as f64 / 60.0 / 60.0 / 24.0 / 30.0),
        ("sec", "yr") => Ok(val as f64 / 60.0 / 60.0 / 24.0 / 365.0),
        ("sec", "dec") => Ok(val as f64 / 10.0 / 60.0 / 60.0 / 24.0 / 365.0),

        ("min", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0),
        ("min", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0),
        ("min", "ms") => Ok(val as f64 * 1000.0 * 60.0),
        ("min", "sec") => Ok(val as f64 * 60.0),
        ("min", "min") => Ok(val as f64),
        ("min", "hr") => Ok(val as f64 / 60.0),
        ("min", "day") => Ok(val as f64 / 60.0 / 24.0),
        ("min", "wk") => Ok(val as f64 / 60.0 / 24.0 / 7.0),
        ("min", "month") => Ok(val as f64 / 60.0 / 24.0 / 30.0),
        ("min", "yr") => Ok(val as f64 / 60.0 / 24.0 / 365.0),
        ("min", "dec") => Ok(val as f64 / 10.0 / 60.0 / 24.0 / 365.0),

        ("hr", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0 * 60.0),
        ("hr", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0 * 60.0),
        ("hr", "ms") => Ok(val as f64 * 1000.0 * 60.0 * 60.0),
        ("hr", "sec") => Ok(val as f64 * 60.0 * 60.0),
        ("hr", "min") => Ok(val as f64 * 60.0),
        ("hr", "hr") => Ok(val as f64),
        ("hr", "day") => Ok(val as f64 / 24.0),
        ("hr", "wk") => Ok(val as f64 / 24.0 / 7.0),
        ("hr", "month") => Ok(val as f64 / 24.0 / 30.0),
        ("hr", "yr") => Ok(val as f64 / 24.0 / 365.0),
        ("hr", "dec") => Ok(val as f64 / 10.0 / 24.0 / 365.0),

        ("day", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0),
        ("day", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0),
        ("day", "ms") => Ok(val as f64 * 1000.0 * 60.0 * 60.0 * 24.0),
        ("day", "sec") => Ok(val as f64 * 60.0 * 60.0 * 24.0),
        ("day", "min") => Ok(val as f64 * 60.0 * 24.0),
        ("day", "hr") => Ok(val as f64 * 24.0),
        ("day", "day") => Ok(val as f64),
        ("day", "wk") => Ok(val as f64 / 7.0),
        ("day", "month") => Ok(val as f64 / 30.0),
        ("day", "yr") => Ok(val as f64 / 365.0),
        ("day", "dec") => Ok(val as f64 / 10.0 / 365.0),

        ("wk", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 7.0),
        ("wk", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 7.0),
        ("wk", "ms") => Ok(val as f64 * 1000.0 * 60.0 * 60.0 * 24.0 * 7.0),
        ("wk", "sec") => Ok(val as f64 * 60.0 * 60.0 * 24.0 * 7.0),
        ("wk", "min") => Ok(val as f64 * 60.0 * 24.0 * 7.0),
        ("wk", "hr") => Ok(val as f64 * 24.0 * 7.0),
        ("wk", "day") => Ok(val as f64 * 7.0),
        ("wk", "wk") => Ok(val as f64),
        ("wk", "month") => Ok(val as f64 / 4.0),
        ("wk", "yr") => Ok(val as f64 / 52.0),
        ("wk", "dec") => Ok(val as f64 / 10.0 / 52.0),

        ("month", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 30.0),
        ("month", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 30.0),
        ("month", "ms") => Ok(val as f64 * 1000.0 * 60.0 * 60.0 * 24.0 * 30.0),
        ("month", "sec") => Ok(val as f64 * 60.0 * 60.0 * 24.0 * 30.0),
        ("month", "min") => Ok(val as f64 * 60.0 * 24.0 * 30.0),
        ("month", "hr") => Ok(val as f64 * 24.0 * 30.0),
        ("month", "day") => Ok(val as f64 * 30.0),
        ("month", "wk") => Ok(val as f64 * 4.0),
        ("month", "month") => Ok(val as f64),
        ("month", "yr") => Ok(val as f64 / 12.0),
        ("month", "dec") => Ok(val as f64 / 10.0 / 12.0),

        ("yr", "ns") => Ok(val as f64 * 1000.0 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 365.0),
        ("yr", "us") => Ok(val as f64 * 1000.0 * 1000.0 * 60.0 * 60.0 * 24.0 * 365.0),
        ("yr", "ms") => Ok(val as f64 * 1000.0 * 60.0 * 60.0 * 24.0 * 365.0),
        ("yr", "sec") => Ok(val as f64 * 60.0 * 60.0 * 24.0 * 365.0),
        ("yr", "min") => Ok(val as f64 * 60.0 * 24.0 * 365.0),
        ("yr", "hr") => Ok(val as f64 * 24.0 * 365.0),
        ("yr", "day") => Ok(val as f64 * 365.0),
        ("yr", "wk") => Ok(val as f64 * 52.0),
        ("yr", "month") => Ok(val as f64 * 12.0),
        ("yr", "yr") => Ok(val as f64),
        ("yr", "dec") => Ok(val as f64 / 10.0),

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

fn action(
    input: &Value,
    convert_to_unit: &Option<Spanned<String>>,
    float_precision: usize,
    span: Span,
) -> Value {
    match input {
        Value::Duration {
            val: val_num,
            span: value_span,
        } => {
            if let Some(to_unit) = convert_to_unit {
                let from_unit = "ns";
                let duration = *val_num;
                match convert_str_from_unit_to_unit(
                    duration,
                    from_unit,
                    &to_unit.item,
                    span,
                    *value_span,
                ) {
                    Ok(d) => {
                        if d.fract() == 0.0 {
                            Value::String {
                                val: format!("{} {}", d, &to_unit.item),
                                span: *value_span,
                            }
                        } else {
                            Value::String {
                                val: format!("{:.float_precision$} {}", d, &to_unit.item),
                                span: *value_span,
                            }
                        }
                    }
                    Err(e) => Value::Error { error: e },
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
                        Ok(d) => {
                            if d.fract() == 0.0 {
                                Value::String {
                                    val: format!("{} {}", d, &to_unit.item),
                                    span: *value_span,
                                }
                            } else {
                                Value::String {
                                    val: format!("{:.float_precision$} {}", d, &to_unit.item),
                                    span: *value_span,
                                }
                            }
                        }
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
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

        let actual = action(&word, &convert_duration, 2, span);
        assert_eq!(actual, expected);
    }
}
