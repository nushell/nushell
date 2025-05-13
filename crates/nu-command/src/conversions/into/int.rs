use std::str::FromStr;

use chrono::{FixedOffset, TimeZone};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{all_supported_units, FilesizeUnit, Unit};

use nu_utils::get_system_locale;

struct Arguments {
    radix: u32,
    cell_paths: Option<Vec<CellPath>>,
    signed: bool,
    little_endian: bool,
    unit: Option<Spanned<Unit>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct IntoInt;

impl Command for IntoInt {
    fn name(&self) -> &str {
        "into int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into int")
            .input_output_types(vec![
                (Type::String, Type::Int),
                (Type::Number, Type::Int),
                (Type::Bool, Type::Int),
                // Unix timestamp in nanoseconds
                (Type::Date, Type::Int),
                (Type::Duration, Type::Int),
                (Type::Filesize, Type::Int),
                (Type::Binary, Type::Int),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Bool)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Filesize)),
                    Type::List(Box::new(Type::Int)),
                ),
                // Relaxed case to support heterogeneous lists
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Int)),
                ),
            ])
            .allow_variants_without_examples(true)
            .named("radix", SyntaxShape::Number, "radix of integer", Some('r'))
            .named(
                "endian",
                SyntaxShape::String,
                "byte encode endian, available options: native(default), little, big",
                Some('e'),
            )
            .named(
                "unit",
                SyntaxShape::String,
                "the unit of the duration or filesize output",
                Some('u'),
            )
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert value to integer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "natural"]
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

        let radix = call.get_flag::<Value>(engine_state, stack, "radix")?;
        let radix: u32 = match radix {
            Some(val) => {
                let span = val.span();
                match val {
                    Value::Int { val, .. } => {
                        if !(2..=36).contains(&val) {
                            return Err(ShellError::TypeMismatch {
                                err_message: "Radix must lie in the range [2, 36]".to_string(),
                                span,
                            });
                        }
                        val as u32
                    }
                    _ => 10,
                }
            }
            None => 10,
        };

        let endian = call.get_flag::<Value>(engine_state, stack, "endian")?;
        let little_endian = match endian {
            Some(val) => {
                let span = val.span();
                match val {
                    Value::String { val, .. } => match val.as_str() {
                        "native" => cfg!(target_endian = "little"),
                        "little" => true,
                        "big" => false,
                        _ => {
                            return Err(ShellError::TypeMismatch {
                                err_message: "Endian must be one of native, little, big"
                                    .to_string(),
                                span,
                            });
                        }
                    },
                    _ => false,
                }
            }
            None => cfg!(target_endian = "little"),
        };

        let signed = call.has_flag(engine_state, stack, "signed")?;

        let unit = match call.get_flag::<Spanned<String>>(engine_state, stack, "unit")? {
            Some(spanned_unit) => {
                let parsed_filesize_unit = FilesizeUnit::from_str(&spanned_unit.item);
                let parsed_unit = {
                    match parsed_filesize_unit {
                        Ok(u) => Unit::Filesize(u),
                        Err(_) => match spanned_unit.item.as_str() {
                            "ns" => Unit::Nanosecond,
                            "us" | "µs" => Unit::Microsecond,
                            "ms" => Unit::Millisecond,
                            "sec" => Unit::Second,
                            "min" => Unit::Minute,
                            "hr" => Unit::Hour,
                            "day" => Unit::Day,
                            "wk" => Unit::Week,
                            _ => {
                                return Err(ShellError::InvalidUnit {
                                    supported_units: all_supported_units(),
                                    span: spanned_unit.span,
                                });
                            }
                        },
                    }
                };
                Some(Spanned {
                    item: parsed_unit,
                    span: spanned_unit.span,
                })
            }
            None => None,
        };

        let args = Arguments {
            radix,
            little_endian,
            signed,
            cell_paths,
            unit,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to int in table",
                example: "[[num]; ['-5'] [4] [1.5]] | into int num",
                result: None,
            },
            Example {
                description: "Convert string to int",
                example: "'2' | into int",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Convert float to int",
                example: "5.9 | into int",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Convert decimal string to int",
                example: "'5.9' | into int",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Convert file size to int",
                example: "4KB | into int",
                result: Some(Value::test_int(4000)),
            },
            Example {
                description: "Convert bool to int",
                example: "[false, true] | into int",
                result: Some(Value::list(
                    vec![Value::test_int(0), Value::test_int(1)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Convert date to int (Unix nanosecond timestamp)",
                example: "1983-04-13T12:09:14.123456789-05:00 | into int",
                result: Some(Value::test_int(419101754123456789)),
            },
            Example {
                description: "Convert to int from binary data (radix: 2)",
                example: "'1101' | into int --radix 2",
                result: Some(Value::test_int(13)),
            },
            Example {
                description: "Convert to int from hex",
                example: "'FF' |  into int --radix 16",
                result: Some(Value::test_int(255)),
            },
            Example {
                description: "Convert octal string to int",
                example: "'0o10132' | into int",
                result: Some(Value::test_int(4186)),
            },
            Example {
                description: "Convert 0 padded string to int",
                example: "'0010132' | into int",
                result: Some(Value::test_int(10132)),
            },
            Example {
                description: "Convert 0 padded string to int with radix 8",
                example: "'0010132' | into int --radix 8",
                result: Some(Value::test_int(4186)),
            },
            Example {
                description: "Convert binary value to int",
                example: "0x[10] | into int",
                result: Some(Value::test_int(16)),
            },
            Example {
                description: "Convert binary value to signed int",
                example: "0x[a0] | into int --signed",
                result: Some(Value::test_int(-96)),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let radix = args.radix;
    let signed = args.signed;
    let little_endian = args.little_endian;
    let unit_option = &args.unit;
    let val_span = input.span();

    if let Some(spanned_unit) = unit_option {
        let unit_str = spanned_unit.item.as_str();
        return match spanned_unit.item.build_value(1, head) {
            Ok(val_one_in_unit) => {
                match (input, &val_one_in_unit) {
                    (Value::Duration { .. }, Value::Duration { .. }) => input
                        .div(input.span(), &val_one_in_unit, val_one_in_unit.span())
                        .unwrap_or_else(|err| Value::error(err, head)),
                    (Value::Filesize { .. }, Value::Filesize { .. }) => input
                        .div(input.span(), &val_one_in_unit, val_one_in_unit.span())
                        .unwrap_or_else(|err| Value::error(err, head)),
                    (other_from, _) => {
                        let target_unit_type = match spanned_unit.item {
                            Unit::Filesize(..) => "filesize",
                            _ => "duration",
                        };
                        let from_type = other_from.get_type();
                        let help_msg = if ["filesize", "duration"]
                            .contains(&from_type.to_string().as_str())
                        {
                            format!(
                                "either the input value should be a {} or choose a {} unit",
                                target_unit_type, from_type
                            )
                        } else {
                            format!("the input value should be a {}. The {} type has no associated units.", target_unit_type, from_type)
                        };
                        Value::error(
                            ShellError::CantConvertToUnit {
                                target_unit: unit_str.to_string(),
                                to_type: target_unit_type.to_string(),
                                from_type: from_type.to_string(),
                                span: other_from.span(),
                                unit_span: spanned_unit.span,
                                help: Some(help_msg),
                            },
                            head,
                        )
                    }
                }
            }
            Err(err) => Value::error(err, head),
        };
    };

    match input {
        Value::Int { val: _, .. } => {
            if radix == 10 {
                input.clone()
            } else {
                convert_int(input, head, radix)
            }
        }
        Value::Filesize { val, .. } => Value::int(val.get(), head),
        Value::Float { val, .. } => Value::int(
            {
                if radix == 10 {
                    *val as i64
                } else {
                    match convert_int(&Value::int(*val as i64, head), head, radix).as_int() {
                        Ok(v) => v,
                        _ => {
                            return Value::error(
                                ShellError::CantConvert {
                                    to_type: "float".to_string(),
                                    from_type: "int".to_string(),
                                    span: head,
                                    help: None,
                                },
                                head,
                            );
                        }
                    }
                }
            },
            head,
        ),
        Value::String { val, .. } => {
            if radix == 10 {
                match int_from_string(val, head) {
                    Ok(val) => Value::int(val, head),
                    Err(error) => Value::error(error, head),
                }
            } else {
                convert_int(input, head, radix)
            }
        }
        Value::Bool { val, .. } => {
            if *val {
                Value::int(1, head)
            } else {
                Value::int(0, head)
            }
        }
        Value::Date { val, .. } => {
            if val
                < &FixedOffset::east_opt(0)
                    .expect("constant")
                    .with_ymd_and_hms(1677, 9, 21, 0, 12, 44)
                    .unwrap()
                || val
                    > &FixedOffset::east_opt(0)
                        .expect("constant")
                        .with_ymd_and_hms(2262, 4, 11, 23, 47, 16)
                        .unwrap()
            {
                Value::error (
                    ShellError::IncorrectValue {
                        msg: "DateTime out of range for timestamp: 1677-09-21T00:12:43Z to 2262-04-11T23:47:16".to_string(),
                        val_span,
                        call_span: head,
                    },
                    head,
                )
            } else {
                Value::int(val.timestamp_nanos_opt().unwrap_or_default(), head)
            }
        }
        Value::Duration { val, .. } => Value::int(*val, head),
        Value::Binary { val, .. } => {
            use byteorder::{BigEndian, ByteOrder, LittleEndian};

            let mut val = val.to_vec();
            let size = val.len();

            if size == 0 {
                return Value::int(0, head);
            }

            if size > 8 {
                return Value::error(
                    ShellError::IncorrectValue {
                        msg: format!("binary input is too large to convert to int ({size} bytes)"),
                        val_span,
                        call_span: head,
                    },
                    head,
                );
            }

            match (little_endian, signed) {
                (true, true) => Value::int(LittleEndian::read_int(&val, size), head),
                (false, true) => Value::int(BigEndian::read_int(&val, size), head),
                (true, false) => {
                    while val.len() < 8 {
                        val.push(0);
                    }
                    val.resize(8, 0);

                    Value::int(LittleEndian::read_i64(&val), head)
                }
                (false, false) => {
                    while val.len() < 8 {
                        val.insert(0, 0);
                    }
                    val.resize(8, 0);

                    Value::int(BigEndian::read_i64(&val), head)
                }
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int, float, filesize, date, string, binary, duration, or bool"
                    .into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

fn convert_int(input: &Value, head: Span, radix: u32) -> Value {
    let i = match input {
        Value::Int { val, .. } => val.to_string(),
        Value::String { val, .. } => {
            let val = val.trim();
            if val.starts_with("0x") // hex
                || val.starts_with("0b") // binary
                || val.starts_with("0o")
            // octal
            {
                match int_from_string(val, head) {
                    Ok(x) => return Value::int(x, head),
                    Err(e) => return Value::error(e, head),
                }
            } else if val.starts_with("00") {
                // It's a padded string
                match i64::from_str_radix(val, radix) {
                    Ok(n) => return Value::int(n, head),
                    Err(e) => {
                        return Value::error(
                            ShellError::CantConvert {
                                to_type: "string".to_string(),
                                from_type: "int".to_string(),
                                span: head,
                                help: Some(e.to_string()),
                            },
                            head,
                        );
                    }
                }
            }
            val.to_string()
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => return input.clone(),
        other => {
            return Value::error(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string and int".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                },
                head,
            );
        }
    };
    match i64::from_str_radix(i.trim(), radix) {
        Ok(n) => Value::int(n, head),
        Err(_reason) => Value::error(
            ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "int".to_string(),
                span: head,
                help: None,
            },
            head,
        ),
    }
}

fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    // Get the Locale so we know what the thousands separator is
    let locale = get_system_locale();

    // Now that we know the locale, get the thousands separator and remove it
    // so strings like 1,123,456 can be parsed as 1123456
    let no_comma_string = a_string.replace(locale.separator(), "");

    let trimmed = no_comma_string.trim();
    match trimmed {
        b if b.starts_with("0b") => {
            let num = match i64::from_str_radix(b.trim_start_matches("0b"), 2) {
                Ok(n) => n,
                Err(_reason) => {
                    return Err(ShellError::CantConvert {
                        to_type: "int".to_string(),
                        from_type: "string".to_string(),
                        span,
                        help: Some(r#"digits following "0b" can only be 0 or 1"#.to_string()),
                    });
                }
            };
            Ok(num)
        }
        h if h.starts_with("0x") => {
            let num =
                match i64::from_str_radix(h.trim_start_matches("0x"), 16) {
                    Ok(n) => n,
                    Err(_reason) => return Err(ShellError::CantConvert {
                        to_type: "int".to_string(),
                        from_type: "string".to_string(),
                        span,
                        help: Some(
                            r#"hexadecimal digits following "0x" should be in 0-9, a-f, or A-F"#
                                .to_string(),
                        ),
                    }),
                };
            Ok(num)
        }
        o if o.starts_with("0o") => {
            let num = match i64::from_str_radix(o.trim_start_matches("0o"), 8) {
                Ok(n) => n,
                Err(_reason) => {
                    return Err(ShellError::CantConvert {
                        to_type: "int".to_string(),
                        from_type: "string".to_string(),
                        span,
                        help: Some(r#"octal digits following "0o" should be in 0-7"#.to_string()),
                    });
                }
            };
            Ok(num)
        }
        _ => match trimmed.parse::<i64>() {
            Ok(n) => Ok(n),
            Err(_) => match a_string.parse::<f64>() {
                Ok(f) => Ok(f as i64),
                _ => Err(ShellError::CantConvert {
                    to_type: "int".to_string(),
                    from_type: "string".to_string(),
                    span,
                    help: Some(format!(
                        r#"string "{trimmed}" does not represent a valid integer"#
                    )),
                }),
            },
        },
    }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, FixedOffset};
    use rstest::rstest;

    use super::Value;
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoInt {})
    }

    #[test]
    fn turns_to_integer() {
        let word = Value::test_string("10");
        let expected = Value::test_int(10);

        let actual = action(
            &word,
            &Arguments {
                radix: 10,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_binary_to_integer() {
        let s = Value::test_string("0b101");
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, Value::test_int(5));
    }

    #[test]
    fn turns_hex_to_integer() {
        let s = Value::test_string("0xFF");
        let actual = action(
            &s,
            &Arguments {
                radix: 16,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );
        assert_eq!(actual, Value::test_int(255));
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_integerlike_string() {
        let integer_str = Value::test_string("36anra");

        let actual = action(
            &integer_str,
            &Arguments {
                radix: 10,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );

        assert_eq!(actual.get_type(), Error)
    }

    #[rstest]
    #[case("2262-04-11T23:47:16+00:00", 0x7fff_ffff_ffff_ffff)]
    #[case("1970-01-01T00:00:00+00:00", 0)]
    #[case("1677-09-21T00:12:44+00:00", -0x7fff_ffff_ffff_ffff)]
    fn datetime_to_int_values_that_work(
        #[case] dt_in: DateTime<FixedOffset>,
        #[case] int_expected: i64,
    ) {
        let s = Value::test_date(dt_in);
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );
        // ignore fractional seconds -- I don't want to hard code test values that might vary due to leap nanoseconds.
        let exp_truncated = (int_expected / 1_000_000_000) * 1_000_000_000;
        assert_eq!(actual, Value::test_int(exp_truncated));
    }

    #[rstest]
    #[case("2262-04-11T23:47:17+00:00", "DateTime out of range for timestamp")]
    #[case("1677-09-21T00:12:43+00:00", "DateTime out of range for timestamp")]
    fn datetime_to_int_values_that_fail(
        #[case] dt_in: DateTime<FixedOffset>,
        #[case] err_expected: &str,
    ) {
        let s = Value::test_date(dt_in);
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                signed: false,
                little_endian: false,
                unit: None,
            },
            Span::test_data(),
        );
        if let Value::Error { error, .. } = actual {
            if let ShellError::IncorrectValue { msg: e, .. } = *error {
                assert!(
                    e.contains(err_expected),
                    "{e:?} doesn't contain {err_expected}"
                );
            } else {
                panic!("Unexpected error variant {error:?}")
            }
        } else {
            panic!("Unexpected actual value {actual:?}")
        }
    }
}
