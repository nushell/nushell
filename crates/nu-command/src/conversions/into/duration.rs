use nu_engine::command_prelude::*;
use nu_parser::parse_unit_value;
use nu_protocol::ast::DurationUnit;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .input_output_types(vec![
                (Type::Int, Type::Duration),
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

    fn usage(&self) -> &str {
        "Convert value to duration."
    }

    fn extra_usage(&self) -> &str {
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
                result: Some(Value::test_duration(
                    7 * DurationUnit::Minute.as_nanos_i64(),
                )),
            },
            Example {
                description: "Convert compound duration string to duration value",
                example: "'1day 2hr 3min 4sec' | into duration",
                result: Some(Value::test_duration(
                    DurationUnit::Day.as_nanos_i64()
                        + 2 * DurationUnit::Hour.as_nanos_i64()
                        + 3 * DurationUnit::Minute.as_nanos_i64()
                        + 4 * DurationUnit::Second.as_nanos_i64(),
                )),
            },
            Example {
                description: "Convert table of duration strings to table of duration values",
                example:
                    "[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_duration(DurationUnit::Second.as_nanos_i64()),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(2 * DurationUnit::Minute.as_nanos_i64()),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(3 * DurationUnit::Hour.as_nanos_i64()),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(4 * DurationUnit::Day.as_nanos_i64()),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_duration(5 * DurationUnit::Week.as_nanos_i64()),
                    }),
                ])),
            },
            Example {
                description: "Convert duration to duration",
                example: "420sec | into duration",
                result: Some(Value::test_duration(
                    420 * DurationUnit::Second.as_nanos_i64(),
                )),
            },
            Example {
                description: "Convert a number of ns to duration",
                example: "1_234_567 | into duration",
                result: Some(Value::test_duration(1_234_567)),
            },
            Example {
                description: "Convert a number of an arbitrary unit to duration",
                example: "1_234 | into duration --unit ms",
                result: Some(Value::test_duration(
                    1_234 * DurationUnit::Millisecond.as_nanos_i64(),
                )),
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
    let span = input.span().unwrap_or(call.head);
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    let unit = match call.get_flag::<String>(engine_state, stack, "unit")? {
        Some(sep) => {
            sep.parse::<DurationUnit>()
                .map_err(|expected| ShellError::CantConvertToDuration {
                    details: sep,
                    dst_span: span,
                    src_span: span,
                    help: Some(expected.into()),
                })?
        }
        None => DurationUnit::Nanosecond,
    };

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, unit, span)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, unit, span)),
                    );
                    if let Err(error) = r {
                        return Value::error(error, span);
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

// convert string list of duration values to duration NS.
// technique for getting substrings and span based on: https://stackoverflow.com/a/67098851/2036651
#[inline]
fn addr_of(s: &str) -> usize {
    s.as_ptr() as usize
}

fn split_whitespace_indices(s: &str, span: Span) -> impl Iterator<Item = (&str, Span)> {
    s.split_whitespace().map(move |sub| {
        let start_offset = span.start + addr_of(sub) - addr_of(s);
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
    if let Some(Ok(duration)) = parse_unit_value(s.as_bytes(), span, DurationUnit::as_nanos_i64) {
        return Ok(duration.value);
    }

    Err(ShellError::CantConvertToDuration {
        details: s.to_string(),
        dst_span: span,
        src_span: span,
        help: Some("Maybe the number was too large or the unit was invalid. The supported units are ns, us/µs, ms, sec, min, hr, day, and wk.".into()),
    })
}

fn action(input: &Value, unit: DurationUnit, span: Span) -> Value {
    let value_span = input.span();
    match input {
        Value::Duration { .. } => input.clone(),
        Value::String { val, .. } => match compound_to_duration(val, value_span) {
            Ok(val) => Value::duration(val, span),
            Err(error) => Value::error(error, span),
        },
        Value::Int { val, .. } => Value::duration(*val * unit.as_nanos_i64(), span),
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

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[rstest]
    #[case("3ns", 3)]
    #[case("4us", 4 * DurationUnit::Microsecond.as_nanos_i64())]
    #[case("4\u{00B5}s", 4 * DurationUnit::Microsecond.as_nanos_i64())] // micro sign
    #[case("4\u{03BC}s", 4 * DurationUnit::Microsecond.as_nanos_i64())] // mu symbol
    #[case("5ms", 5 * DurationUnit::Millisecond.as_nanos_i64())]
    #[case("1sec", DurationUnit::Second.as_nanos_i64())]
    #[case("7min", 7 * DurationUnit::Minute.as_nanos_i64())]
    #[case("42hr", 42 * DurationUnit::Hour.as_nanos_i64())]
    #[case("123day", 123 * DurationUnit::Day.as_nanos_i64())]
    #[case("3wk", 3 * DurationUnit::Week.as_nanos_i64())]
    #[case("86hr 26ns", 86 * DurationUnit::Hour.as_nanos_i64() + 26 * DurationUnit::Nanosecond.as_nanos_i64())] // compound duration string
    #[case("14ns 3hr 17sec", 14 + 3 * DurationUnit::Hour.as_nanos_i64() + 17 * DurationUnit::Second.as_nanos_i64())] // compound string with units in random order

    fn turns_string_to_duration(#[case] phrase: &str, #[case] expected_duration_val: i64) {
        let actual = action(
            &Value::test_string(phrase),
            DurationUnit::Nanosecond,
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
