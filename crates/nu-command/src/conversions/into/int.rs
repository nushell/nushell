use chrono::{FixedOffset, TimeZone};

use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

struct Arguments {
    radix: u32,
    cell_paths: Option<Vec<CellPath>>,
    little_endian: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
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
            .switch("little-endian", "use little-endian byte decoding", None)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
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

        let radix = call.get_flag::<SpannedValue>(engine_state, stack, "radix")?;
        let radix: u32 = match radix {
            Some(SpannedValue::Int { val, span }) => {
                if !(2..=36).contains(&val) {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Radix must lie in the range [2, 36]".to_string(),
                        span,
                    });
                }
                val as u32
            }
            Some(_) => 10,
            None => 10,
        };
        let args = Arguments {
            radix,
            little_endian: call.has_flag("little-endian"),
            cell_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to integer in table",
                example: "[[num]; ['-5'] [4] [1.5]] | into int num",
                result: None,
            },
            Example {
                description: "Convert string to integer",
                example: "'2' | into int",
                result: Some(SpannedValue::test_int(2)),
            },
            Example {
                description: "Convert decimal to integer",
                example: "5.9 | into int",
                result: Some(SpannedValue::test_int(5)),
            },
            Example {
                description: "Convert decimal string to integer",
                example: "'5.9' | into int",
                result: Some(SpannedValue::test_int(5)),
            },
            Example {
                description: "Convert file size to integer",
                example: "4KB | into int",
                result: Some(SpannedValue::test_int(4000)),
            },
            Example {
                description: "Convert bool to integer",
                example: "[false, true] | into int",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::test_int(0), SpannedValue::test_int(1)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert date to integer (Unix nanosecond timestamp)",
                example: "1983-04-13T12:09:14.123456789-05:00 | into int",
                result: Some(SpannedValue::test_int(419101754123456789)),
            },
            Example {
                description: "Convert to integer from binary",
                example: "'1101' | into int -r 2",
                result: Some(SpannedValue::test_int(13)),
            },
            Example {
                description: "Convert to integer from hex",
                example: "'FF' |  into int -r 16",
                result: Some(SpannedValue::test_int(255)),
            },
            Example {
                description: "Convert octal string to integer",
                example: "'0o10132' | into int",
                result: Some(SpannedValue::test_int(4186)),
            },
            Example {
                description: "Convert 0 padded string to integer",
                example: "'0010132' | into int",
                result: Some(SpannedValue::test_int(10132)),
            },
            Example {
                description: "Convert 0 padded string to integer with radix",
                example: "'0010132' | into int -r 8",
                result: Some(SpannedValue::test_int(4186)),
            },
        ]
    }
}

fn action(input: &SpannedValue, args: &Arguments, span: Span) -> SpannedValue {
    let radix = args.radix;
    let little_endian = args.little_endian;
    match input {
        SpannedValue::Int { val: _, .. } => {
            if radix == 10 {
                input.clone()
            } else {
                convert_int(input, span, radix)
            }
        }
        SpannedValue::Filesize { val, .. } => SpannedValue::Int { val: *val, span },
        SpannedValue::Float { val, .. } => SpannedValue::Int {
            val: {
                if radix == 10 {
                    *val as i64
                } else {
                    match convert_int(
                        &SpannedValue::Int {
                            val: *val as i64,
                            span,
                        },
                        span,
                        radix,
                    )
                    .as_i64()
                    {
                        Ok(v) => v,
                        _ => {
                            return SpannedValue::Error {
                                error: Box::new(ShellError::CantConvert {
                                    to_type: "float".to_string(),
                                    from_type: "integer".to_string(),
                                    span,
                                    help: None,
                                }),
                                span,
                            }
                        }
                    }
                }
            },
            span,
        },
        SpannedValue::String { val, .. } => {
            if radix == 10 {
                match int_from_string(val, span) {
                    Ok(val) => SpannedValue::Int { val, span },
                    Err(error) => SpannedValue::Error {
                        error: Box::new(error),
                        span,
                    },
                }
            } else {
                convert_int(input, span, radix)
            }
        }
        SpannedValue::Bool { val, .. } => {
            if *val {
                SpannedValue::Int { val: 1, span }
            } else {
                SpannedValue::Int { val: 0, span }
            }
        }
        SpannedValue::Date { val, .. } => {
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
                SpannedValue::Error {
                    error: Box::new(ShellError::IncorrectValue {
                        msg: "DateTime out of range for timestamp: 1677-09-21T00:12:43Z to 2262-04-11T23:47:16".to_string(),
                        span
                    }),
                    span,
                }
            } else {
                SpannedValue::Int {
                    val: val.timestamp_nanos(),
                    span,
                }
            }
        }
        SpannedValue::Duration { val, .. } => SpannedValue::Int { val: *val, span },
        SpannedValue::Binary { val, span } => {
            use byteorder::{BigEndian, ByteOrder, LittleEndian};

            let mut val = val.to_vec();

            if little_endian {
                while val.len() < 8 {
                    val.push(0);
                }
                val.resize(8, 0);

                SpannedValue::int(LittleEndian::read_i64(&val), *span)
            } else {
                while val.len() < 8 {
                    val.insert(0, 0);
                }
                val.resize(8, 0);

                SpannedValue::int(BigEndian::read_i64(&val), *span)
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => input.clone(),
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "integer, float, filesize, date, string, binary, duration or bool"
                    .into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            }),
            span,
        },
    }
}

fn convert_int(input: &SpannedValue, head: Span, radix: u32) -> SpannedValue {
    let i = match input {
        SpannedValue::Int { val, .. } => val.to_string(),
        SpannedValue::String { val, .. } => {
            let val = val.trim();
            if val.starts_with("0x") // hex
                || val.starts_with("0b") // binary
                || val.starts_with("0o")
            // octal
            {
                match int_from_string(val, head) {
                    Ok(x) => return SpannedValue::int(x, head),
                    Err(e) => {
                        return SpannedValue::Error {
                            error: Box::new(e),
                            span: head,
                        }
                    }
                }
            } else if val.starts_with("00") {
                // It's a padded string
                match i64::from_str_radix(val, radix) {
                    Ok(n) => return SpannedValue::int(n, head),
                    Err(e) => {
                        return SpannedValue::Error {
                            error: Box::new(ShellError::CantConvert {
                                to_type: "string".to_string(),
                                from_type: "int".to_string(),
                                span: head,
                                help: Some(e.to_string()),
                            }),
                            span: head,
                        }
                    }
                }
            }
            val.to_string()
        }
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => return input.clone(),
        other => {
            return SpannedValue::Error {
                error: Box::new(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string and integer".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                }),
                span: head,
            };
        }
    };
    match i64::from_str_radix(i.trim(), radix) {
        Ok(n) => SpannedValue::int(n, head),
        Err(_reason) => SpannedValue::Error {
            error: Box::new(ShellError::CantConvert {
                to_type: "string".to_string(),
                from_type: "int".to_string(),
                span: head,
                help: None,
            }),
            span: head,
        },
    }
}

fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    let trimmed = a_string.trim();
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
                    })
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
                    })
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

    use super::SpannedValue;
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn turns_to_integer() {
        let word = SpannedValue::test_string("10");
        let expected = SpannedValue::test_int(10);

        let actual = action(
            &word,
            &Arguments {
                radix: 10,
                cell_paths: None,
                little_endian: false,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_binary_to_integer() {
        let s = SpannedValue::test_string("0b101");
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                little_endian: false,
            },
            Span::test_data(),
        );
        assert_eq!(actual, SpannedValue::test_int(5));
    }

    #[test]
    fn turns_hex_to_integer() {
        let s = SpannedValue::test_string("0xFF");
        let actual = action(
            &s,
            &Arguments {
                radix: 16,
                cell_paths: None,
                little_endian: false,
            },
            Span::test_data(),
        );
        assert_eq!(actual, SpannedValue::test_int(255));
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_integerlike_string() {
        let integer_str = SpannedValue::test_string("36anra");

        let actual = action(
            &integer_str,
            &Arguments {
                radix: 10,
                cell_paths: None,
                little_endian: false,
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
        let s = SpannedValue::test_date(dt_in);
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                little_endian: false,
            },
            Span::test_data(),
        );
        // ignore fractional seconds -- I don't want to hard code test values that might vary due to leap nanoseconds.
        let exp_truncated = (int_expected / 1_000_000_000) * 1_000_000_000;
        assert_eq!(actual, SpannedValue::test_int(exp_truncated));
    }

    #[rstest]
    #[case("2262-04-11T23:47:17+00:00", "DateTime out of range for timestamp")]
    #[case("1677-09-21T00:12:43+00:00", "DateTime out of range for timestamp")]
    fn datetime_to_int_values_that_fail(
        #[case] dt_in: DateTime<FixedOffset>,
        #[case] err_expected: &str,
    ) {
        let s = SpannedValue::test_date(dt_in);
        let actual = action(
            &s,
            &Arguments {
                radix: 10,
                cell_paths: None,
                little_endian: false,
            },
            Span::test_data(),
        );
        if let SpannedValue::Error { error, .. } = actual {
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
