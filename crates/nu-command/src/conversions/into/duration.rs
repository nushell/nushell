use std::str::FromStr;

use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_parser::{DURATION_UNIT_GROUPS, parse_unit_value};
use nu_protocol::{SUPPORTED_DURATION_UNITS, Unit, ast::Expr};

const NS_PER_US: i64 = 1_000;
const NS_PER_MS: i64 = 1_000_000;
const NS_PER_SEC: i64 = 1_000_000_000;
const NS_PER_MINUTE: i64 = 60 * NS_PER_SEC;
const NS_PER_HOUR: i64 = 60 * NS_PER_MINUTE;
const NS_PER_DAY: i64 = 24 * NS_PER_HOUR;
const NS_PER_WEEK: i64 = 7 * NS_PER_DAY;

const ALLOWED_COLUMNS: [&str; 9] = [
    "week",
    "day",
    "hour",
    "minute",
    "second",
    "millisecond",
    "microsecond",
    "nanosecond",
    "sign",
];
const ALLOWED_SIGNS: [&str; 2] = ["+", "-"];

#[derive(Clone, Debug)]
struct Arguments {
    unit: Option<Spanned<Unit>>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

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
                (Type::record(), Type::record()),
                (Type::record(), Type::Duration),
                (Type::table(), Type::table()),
            ])
            .allow_variants_without_examples(true)
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
        let cell_paths = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

        let unit = match call.get_flag::<Spanned<String>>(engine_state, stack, "unit")? {
            Some(spanned_unit) => match Unit::from_str(&spanned_unit.item) {
                Ok(u) => match u {
                    Unit::Filesize(_) => {
                        return Err(ShellError::InvalidUnit {
                            span: spanned_unit.span,
                            supported_units: SUPPORTED_DURATION_UNITS.join(", "),
                        });
                    }
                    _ => Some(Spanned {
                        item: u,
                        span: spanned_unit.span,
                    }),
                },
                Err(_) => {
                    return Err(ShellError::InvalidUnit {
                        span: spanned_unit.span,
                        supported_units: SUPPORTED_DURATION_UNITS.join(", "),
                    });
                }
            },
            None => None,
        };
        let args = Arguments { unit, cell_paths };
        operate(action, args, input, call.head, engine_state.signals())
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
                example: "[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value",
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
            Example {
                description: "Convert a record to a duration",
                example: "{day: 10, hour: 2, minute: 6, second: 50, sign: '+'} | into duration",
                result: Some(Value::duration(
                    10 * NS_PER_DAY + 2 * NS_PER_HOUR + 6 * NS_PER_MINUTE + 50 * NS_PER_SEC,
                    Span::test_data(),
                )),
            },
        ]
    }
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

    Err(ShellError::InvalidUnit {
        span,
        supported_units: SUPPORTED_DURATION_UNITS.join(", "),
    })
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let value_span = input.span();
    let unit_option = &args.unit;

    if let Value::Record { .. } | Value::Duration { .. } = input {
        if let Some(unit) = unit_option {
            return Value::error(
                ShellError::IncompatibleParameters {
                    left_message: "got a record as input".into(),
                    left_span: head,
                    right_message: "the units should be included in the record".into(),
                    right_span: unit.span,
                },
                head,
            );
        }
    }

    let unit = match unit_option {
        Some(unit) => &unit.item,
        None => &Unit::Nanosecond,
    };

    match input {
        Value::Duration { .. } => input.clone(),
        Value::Record { val, .. } => {
            merge_record(val, head, value_span).unwrap_or_else(|err| Value::error(err, value_span))
        }
        Value::String { val, .. } => {
            if let Ok(num) = val.parse::<f64>() {
                let ns = unit_to_ns_factor(unit);
                return Value::duration((num * (ns as f64)) as i64, head);
            }
            match compound_to_duration(val, value_span) {
                Ok(val) => Value::duration(val, head),
                Err(error) => Value::error(error, head),
            }
        }
        Value::Float { val, .. } => {
            let ns = unit_to_ns_factor(unit);
            Value::duration((*val * (ns as f64)) as i64, head)
        }
        Value::Int { val, .. } => {
            let ns = unit_to_ns_factor(unit);
            Value::duration(*val * ns, head)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string or duration".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

fn merge_record(record: &Record, head: Span, span: Span) -> Result<Value, ShellError> {
    if let Some(invalid_col) = record
        .columns()
        .find(|key| !ALLOWED_COLUMNS.contains(&key.as_str()))
    {
        let allowed_cols = ALLOWED_COLUMNS.join(", ");
        return Err(ShellError::UnsupportedInput {
            msg: format!(
                "Column '{invalid_col}' is not valid for a structured duration. Allowed columns are: {allowed_cols}"
            ),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        });
    };

    let mut duration: i64 = 0;

    if let Some(col_val) = record.get("week") {
        let week = parse_number_from_record(col_val, &head)?;
        duration += week * NS_PER_WEEK;
    };
    if let Some(col_val) = record.get("day") {
        let day = parse_number_from_record(col_val, &head)?;
        duration += day * NS_PER_DAY;
    };
    if let Some(col_val) = record.get("hour") {
        let hour = parse_number_from_record(col_val, &head)?;
        duration += hour * NS_PER_HOUR;
    };
    if let Some(col_val) = record.get("minute") {
        let minute = parse_number_from_record(col_val, &head)?;
        duration += minute * NS_PER_MINUTE;
    };
    if let Some(col_val) = record.get("second") {
        let second = parse_number_from_record(col_val, &head)?;
        duration += second * NS_PER_SEC;
    };
    if let Some(col_val) = record.get("millisecond") {
        let millisecond = parse_number_from_record(col_val, &head)?;
        duration += millisecond * NS_PER_MS;
    };
    if let Some(col_val) = record.get("microsecond") {
        let microsecond = parse_number_from_record(col_val, &head)?;
        duration += microsecond * NS_PER_US;
    };
    if let Some(col_val) = record.get("nanosecond") {
        let nanosecond = parse_number_from_record(col_val, &head)?;
        duration += nanosecond;
    };

    if let Some(sign) = record.get("sign") {
        match sign {
            Value::String { val, .. } => {
                if !ALLOWED_SIGNS.contains(&val.as_str()) {
                    let allowed_signs = ALLOWED_SIGNS.join(", ");
                    return Err(ShellError::IncorrectValue {
                        msg: format!("Invalid sign. Allowed signs are {allowed_signs}").to_string(),
                        val_span: sign.span(),
                        call_span: head,
                    });
                }
                if val == "-" {
                    duration = -duration;
                }
            }
            other => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "int".to_string(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                });
            }
        }
    };

    Ok(Value::duration(duration, span))
}

fn parse_number_from_record(col_val: &Value, head: &Span) -> Result<i64, ShellError> {
    let value = match col_val {
        Value::Int { val, .. } => {
            if *val < 0 {
                return Err(ShellError::IncorrectValue {
                    msg: "number should be positive".to_string(),
                    val_span: col_val.span(),
                    call_span: *head,
                });
            }
            *val
        }
        other => {
            return Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "int".to_string(),
                wrong_type: other.get_type().to_string(),
                dst_span: *head,
                src_span: other.span(),
            });
        }
    };
    Ok(value)
}

fn unit_to_ns_factor(unit: &Unit) -> i64 {
    match unit {
        Unit::Nanosecond => 1,
        Unit::Microsecond => NS_PER_US,
        Unit::Millisecond => NS_PER_MS,
        Unit::Second => NS_PER_SEC,
        Unit::Minute => NS_PER_MINUTE,
        Unit::Hour => NS_PER_HOUR,
        Unit::Day => NS_PER_DAY,
        Unit::Week => NS_PER_WEEK,
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
    #[case("4us", 4 * NS_PER_US)]
    #[case("4\u{00B5}s", 4 * NS_PER_US)] // micro sign
    #[case("4\u{03BC}s", 4 * NS_PER_US)] // mu symbol
    #[case("5ms", 5 * NS_PER_MS)]
    #[case("1sec", NS_PER_SEC)]
    #[case("7min", 7 * NS_PER_MINUTE)]
    #[case("42hr", 42 * NS_PER_HOUR)]
    #[case("123day", 123 * NS_PER_DAY)]
    #[case("3wk", 3 * NS_PER_WEEK)]
    #[case("86hr 26ns", 86 * 3600 * NS_PER_SEC + 26)] // compound duration string
    #[case("14ns 3hr 17sec", 14 + 3 * NS_PER_HOUR + 17 * NS_PER_SEC)] // compound string with units in random order

    fn turns_string_to_duration(#[case] phrase: &str, #[case] expected_duration_val: i64) {
        let args = Arguments {
            unit: Some(Spanned {
                item: Unit::Nanosecond,
                span: Span::test_data(),
            }),
            cell_paths: None,
        };
        let actual = action(&Value::test_string(phrase), &args, Span::test_data());
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
