use nu_engine::command_prelude::*;
use nu_parser::{parse_unit_value, DURATION_UNIT_GROUPS};
use nu_protocol::{ast::Expr, Unit};

const NS_PER_SEC: i64 = 1_000_000_000;
#[derive(Clone)]
pub struct IntoDuration;

impl Command for IntoDuration {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .input_output_types(vec![
                (Type::Int, Type::Duration),
                (Type::Float, Type::Duration),
                (Type::String, Type::Duration),
                (Type::Duration, Type::Duration),
                (Type::table(), Type::table()),
                //todo: record<hour,minute,sign> | into duration -> Duration
                //(Type::record(), Type::record()),
            ])
            //.allow_variants_without_examples(true)
            .named(
                "unit",
                SyntaxShape::String,
                "Unit to convert number into (will have an effect only with integer input)",
                Some('u'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert value to duration."
    }

    fn extra_description(&self) -> &str {
        "Max duration value is i64::MAX nanoseconds; max duration time unit is wk (weeks)."
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
        vec![
            Example {
                description: "Convert duration string to duration value",
                example: "'7min' | into duration",
                result: Some(Value::test_duration(7 * 60 * NS_PER_SEC)),
            },
            Example {
                description: "Convert compound duration string to duration value",
                example: "'1day 2hr 3min 4sec' | into duration",
                result: Some(Value::test_duration(
                    (((((/* 1 * */24) + 2) * 60) + 3) * 60 + 4) * NS_PER_SEC,
                )),
            },
            Example {
                description: "Convert table of duration strings to table of duration values",
                example:
                    "[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_duration(NS_PER_SEC),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(2 * 60 * NS_PER_SEC),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(3 * 60 * 60 * NS_PER_SEC),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(4 * 24 * 60 * 60 * NS_PER_SEC),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(5 * 7 * 24 * 60 * 60 * NS_PER_SEC),
                    }),
                ])),
            },
            Example {
                description: "Convert duration to duration",
                example: "420sec | into duration",
                result: Some(Value::test_duration(7 * 60 * NS_PER_SEC)),
            },
            Example {
                description: "Convert a number of ns to duration",
                example: "1_234_567 | into duration",
                result: Some(Value::test_duration(1_234_567)),
            },
            Example {
                description: "Convert a number of an arbitrary unit to duration",
                example: "1_234 | into duration --unit ms",
                result: Some(Value::test_duration(1_234 * 1_000_000)),
            },
            Example {
                description: "Convert a floating point number of an arbitrary unit to duration",
                example: "1.234 | into duration --unit sec",
                result: Some(Value::test_duration(1_234 * 1_000_000)),
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

    let unit = match call.get_flag::<String>(engine_state, stack, "unit")? {
        Some(sep) => {
            if ["ns", "us", "µs", "ms", "sec", "min", "hr", "day", "wk"]
                .iter()
                .any(|d| d == &sep)
            {
                sep
            } else {
                return Err(ShellError::CantConvertToDuration {
                    details: sep,
                    dst_span: span,
                    src_span: span,
                    help: Some(
                        "supported units are ns, us/µs, ms, sec, min, hr, day, and wk".to_string(),
                    ),
                });
            }
        }
        None => "ns".to_string(),
    };

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &unit.clone(), span)
            } else {
                let unitclone = &unit.clone();
                let mut ret = v;
                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, unitclone, span)),
                    );
                    if let Err(error) = r {
                        return Value::error(error, span);
                    }
                }

                ret
            }
        },
        engine_state.signals(),
    )
}

fn split_whitespace_indices(s: &str, span: Span) -> impl Iterator<Item = (&str, Span)> {
    s.split_whitespace().map(move |sub| {
        // Gets the offset of the `sub` substring inside the string `s`.
        // `wrapping_` operations are necessary because the pointers can
        // overflow on 32-bit platforms.  The result will not overflow, because
        // `sub` is within `s`, and the end of `s` has to be a valid memory
        // address.
        //
        // XXX: this should be replaced with `str::substr_range` from the
        // standard library when it's stabilized.
        let start_offset = span
            .start
            .wrapping_add(sub.as_ptr() as usize)
            .wrapping_sub(s.as_ptr() as usize);
        (sub, Span::new(start_offset, start_offset + sub.len()))
    })
}

fn compound_to_duration(s: &str, span: Span) -> Result<i64, ShellError> {
    let mut duration_ns: i64 = 0;

    for (substring, substring_span) in split_whitespace_indices(s, span) {
        let sub_ns = string_to_duration(substring, substring_span)?;
        duration_ns += sub_ns;
    }

    Ok(duration_ns)
}

fn string_to_duration(s: &str, span: Span) -> Result<i64, ShellError> {
    if let Some(Ok(expression)) = parse_unit_value(
        s.as_bytes(),
        span,
        DURATION_UNIT_GROUPS,
        Type::Duration,
        |x| x,
    ) {
        if let Expr::ValueWithUnit(value) = expression.expr {
            if let Expr::Int(x) = value.expr.expr {
                match value.unit.item {
                    Unit::Nanosecond => return Ok(x),
                    Unit::Microsecond => return Ok(x * 1000),
                    Unit::Millisecond => return Ok(x * 1000 * 1000),
                    Unit::Second => return Ok(x * NS_PER_SEC),
                    Unit::Minute => return Ok(x * 60 * NS_PER_SEC),
                    Unit::Hour => return Ok(x * 60 * 60 * NS_PER_SEC),
                    Unit::Day => return Ok(x * 24 * 60 * 60 * NS_PER_SEC),
                    Unit::Week => return Ok(x * 7 * 24 * 60 * 60 * NS_PER_SEC),
                    _ => {}
                }
            }
        }
    }

    Err(ShellError::CantConvertToDuration {
        details: s.to_string(),
        dst_span: span,
        src_span: span,
        help: Some("supported units are ns, us/µs, ms, sec, min, hr, day, and wk".to_string()),
    })
}

fn action(input: &Value, unit: &str, span: Span) -> Value {
    let value_span = input.span();
    match input {
        Value::Duration { .. } => input.clone(),
        Value::String { val, .. } => {
            if let Ok(num) = val.parse::<f64>() {
                let ns = unit_to_ns_factor(unit);
                return Value::duration((num * (ns as f64)) as i64, span);
            }
            match compound_to_duration(val, value_span) {
                Ok(val) => Value::duration(val, span),
                Err(error) => Value::error(error, span),
            }
        }
        Value::Float { val, .. } => {
            let ns = unit_to_ns_factor(unit);
            Value::duration((*val * (ns as f64)) as i64, span)
        }
        Value::Int { val, .. } => {
            let ns = unit_to_ns_factor(unit);
            Value::duration(*val * ns, span)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string or duration".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

fn unit_to_ns_factor(unit: &str) -> i64 {
    match unit {
        "ns" => 1,
        "us" | "µs" => 1_000,
        "ms" => 1_000_000,
        "sec" => NS_PER_SEC,
        "min" => NS_PER_SEC * 60,
        "hr" => NS_PER_SEC * 60 * 60,
        "day" => NS_PER_SEC * 60 * 60 * 24,
        "wk" => NS_PER_SEC * 60 * 60 * 24 * 7,
        _ => 0,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoDuration {})
    }

    const NS_PER_SEC: i64 = 1_000_000_000;

    #[rstest]
    #[case("3ns", 3)]
    #[case("4us", 4*1000)]
    #[case("4\u{00B5}s", 4*1000)] // micro sign
    #[case("4\u{03BC}s", 4*1000)] // mu symbol
    #[case("5ms", 5 * 1000 * 1000)]
    #[case("1sec", NS_PER_SEC)]
    #[case("7min", 7 * 60 * NS_PER_SEC)]
    #[case("42hr", 42 * 60 * 60 * NS_PER_SEC)]
    #[case("123day", 123 * 24 * 60 * 60 * NS_PER_SEC)]
    #[case("3wk", 3 * 7 * 24 * 60 * 60 * NS_PER_SEC)]
    #[case("86hr 26ns", 86 * 3600 * NS_PER_SEC + 26)] // compound duration string
    #[case("14ns 3hr 17sec", 14 + 3 * 3600 * NS_PER_SEC + 17 * NS_PER_SEC)] // compound string with units in random order

    fn turns_string_to_duration(#[case] phrase: &str, #[case] expected_duration_val: i64) {
        let actual = action(
            &Value::test_string(phrase),
            "ns",
            Span::new(0, phrase.len()),
        );
        match actual {
            Value::Duration {
                val: observed_val, ..
            } => {
                assert_eq!(expected_duration_val, observed_val, "expected != observed")
            }
            other => {
                panic!("Expected Value::Duration, observed {other:?}");
            }
        }
    }
}
