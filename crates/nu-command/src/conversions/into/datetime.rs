use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use human_date_parser::{from_human_time, ParseResult};
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;

const HOUR: i32 = 60 * 60;

#[derive(Clone, Debug)]
struct Arguments {
    zone_options: Option<Spanned<Zone>>,
    format_options: Option<DatetimeFormat>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

// In case it may be confused with chrono::TimeZone
#[derive(Clone, Debug)]
enum Zone {
    Utc,
    Local,
    East(u8),
    West(u8),
    Error, // we want Nushell to cast it instead of Rust
}

impl Zone {
    fn new(i: i64) -> Self {
        if i.abs() <= 12 {
            // guaranteed here
            if i >= 0 {
                Self::East(i as u8) // won't go out of range
            } else {
                Self::West(-i as u8) // same here
            }
        } else {
            Self::Error // Out of range
        }
    }
    fn from_string(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "utc" | "u" => Self::Utc,
            "local" | "l" => Self::Local,
            _ => Self::Error,
        }
    }
}

#[derive(Clone)]
pub struct IntoDatetime;

impl Command for IntoDatetime {
    fn name(&self) -> &str {
        "into datetime"
    }

    fn signature(&self) -> Signature {
        Signature::build("into datetime")
        .input_output_types(vec![
            (Type::Date, Type::Date),
            (Type::Int, Type::Date),
            (Type::String, Type::Date),
            (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Date))),
            (Type::table(), Type::table()),
            (Type::record(), Type::record()),
            (Type::Nothing, Type::table()),
            // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
            // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
            // only applicable for --list flag
            (Type::Any, Type::table()),
        ])
        .allow_variants_without_examples(true)
        .named(
                "timezone",
                SyntaxShape::String,
                "Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' ('u') or 'LOCAL' ('l')",
                Some('z'),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', '-4'",
                Some('o'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify expected format of INPUT string to parse to datetime. Use --list to see options",
                Some('f'),
            )
            .switch(
                "list",
                "Show all possible variables for use in --format flag",
                Some('l'),
                )
            .switch(
                "list-human",
                "Show human-readable datetime parsing examples",
                Some('n'),
                )
            .rest(
            "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if call.has_flag(engine_state, stack, "list")? {
            Ok(generate_strftime_list(call.head, true).into_pipeline_data())
        } else if call.has_flag(engine_state, stack, "list-human")? {
            Ok(list_human_readable_examples(call.head).into_pipeline_data())
        } else {
            let cell_paths = call.rest(engine_state, stack, 0)?;
            let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

            // if zone-offset is specified, then zone will be neglected
            let timezone = call.get_flag::<Spanned<String>>(engine_state, stack, "timezone")?;
            let zone_options =
                match &call.get_flag::<Spanned<i64>>(engine_state, stack, "offset")? {
                    Some(zone_offset) => Some(Spanned {
                        item: Zone::new(zone_offset.item),
                        span: zone_offset.span,
                    }),
                    None => timezone.as_ref().map(|zone| Spanned {
                        item: Zone::from_string(&zone.item),
                        span: zone.span,
                    }),
                };

            let format_options = call
                .get_flag::<String>(engine_state, stack, "format")?
                .as_ref()
                .map(|fmt| DatetimeFormat(fmt.to_string()));

            let args = Arguments {
                format_options,
                zone_options,
                cell_paths,
            };
            operate(action, args, input, call.head, engine_state.signals())
        }
    }

    fn description(&self) -> &str {
        "Convert text or timestamp into a datetime."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "timezone", "UTC"]
    }

    fn examples(&self) -> Vec<Example> {
        let example_result_1 = |nanos: i64| {
            Some(Value::date(
                Utc.timestamp_nanos(nanos).into(),
                Span::test_data(),
            ))
        };
        vec![
            Example {
                description: "Convert timestamp string to datetime with timezone offset",
                example: "'27.02.2021 1:55 pm +0000' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434100_000000000),
            },
            Example {
                description: "Convert standard timestamp string to datetime with timezone offset",
                example: "'2021-02-27T13:55:40.2246+00:00' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_224600000),
            },
            Example {
                description:
                    "Convert non-standard timestamp string, with timezone offset, to datetime using a custom format",
                example: "'20210227_135540+0000' | into datetime --format '%Y%m%d_%H%M%S%z'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Convert non-standard timestamp string, without timezone offset, to datetime with custom formatting",
                example: "'16.11.1984 8:00 am' | into datetime --format '%d.%m.%Y %H:%M %P'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: Some(Value::date(
                    Local
                        .from_local_datetime(
                            &NaiveDateTime::parse_from_str("16.11.1984 8:00 am", "%d.%m.%Y %H:%M %P")
                                .expect("date calculation should not fail in test"),
                        )
                        .unwrap()
                        .with_timezone(Local::now().offset()),
                    Span::test_data(),
                )),
            },
            Example {
                description:
                    "Convert nanosecond-precision unix timestamp to a datetime with offset from UTC",
                example: "1614434140123456789 | into datetime --offset -5",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_123456789),
            },
            Example {
                description: "Convert standard (seconds) unix timestamp to a UTC datetime",
                example: "1614434140 | into datetime -f '%s'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Using a datetime as input simply returns the value",
                example: "2021-02-27T13:55:40 | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Convert list of timestamps to datetimes",
                example: r#"["2023-03-30 10:10:07 -05:00", "2023-05-05 13:43:49 -05:00", "2023-06-05 01:37:42 -05:00"] | into datetime"#,
                result: Some(Value::list(
                    vec![
                        Value::date(
                            DateTime::parse_from_str(
                                "2023-03-30 10:10:07 -05:00",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_from_str(
                                "2023-05-05 13:43:49 -05:00",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_from_str(
                                "2023-06-05 01:37:42 -05:00",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Parsing human readable datetimes",
                example: "'Today at 18:30' | into datetime",
                result: None,
            },
            Example {
                description: "Parsing human readable datetimes",
                example: "'Last Friday at 19:45' | into datetime",
                result: None,
            },
            Example {
                description: "Parsing human readable datetimes",
                example: "'In 5 minutes and 30 seconds' | into datetime",
                result: None,
            },
        ]
    }
}

#[derive(Clone, Debug)]
struct DatetimeFormat(String);

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let timezone = &args.zone_options;
    let dateformat = &args.format_options;

    // noop if the input is already a datetime
    if matches!(input, Value::Date { .. }) {
        return input.clone();
    }

    // Let's try dtparse first
    if matches!(input, Value::String { .. }) && dateformat.is_none() {
        let span = input.span();
        if let Ok(input_val) = input.coerce_str() {
            match parse_date_from_string(&input_val, span) {
                Ok(date) => return Value::date(date, span),
                Err(_) => {
                    if let Ok(date) = from_human_time(&input_val, Local::now().naive_local()) {
                        match date {
                            ParseResult::Date(date) => {
                                let time = Local::now().time();
                                let combined = date.and_time(time);
                                let local_offset = *Local::now().offset();
                                let dt_fixed =
                                    TimeZone::from_local_datetime(&local_offset, &combined)
                                        .single()
                                        .unwrap_or_default();
                                return Value::date(dt_fixed, span);
                            }
                            ParseResult::DateTime(date) => {
                                let local_offset = *Local::now().offset();
                                let dt_fixed = match local_offset.from_local_datetime(&date) {
                                    chrono::LocalResult::Single(dt) => dt,
                                    chrono::LocalResult::Ambiguous(_, _) => {
                                        return Value::error(
                                            ShellError::DatetimeParseError {
                                                msg: "Ambiguous datetime".to_string(),
                                                span,
                                            },
                                            span,
                                        );
                                    }
                                    chrono::LocalResult::None => {
                                        return Value::error(
                                            ShellError::DatetimeParseError {
                                                msg: "Invalid datetime".to_string(),
                                                span,
                                            },
                                            span,
                                        );
                                    }
                                };
                                return Value::date(dt_fixed, span);
                            }
                            ParseResult::Time(time) => {
                                let date = Local::now().date_naive();
                                let combined = date.and_time(time);
                                let local_offset = *Local::now().offset();
                                let dt_fixed =
                                    TimeZone::from_local_datetime(&local_offset, &combined)
                                        .single()
                                        .unwrap_or_default();
                                return Value::date(dt_fixed, span);
                            }
                        }
                    }
                }
            };
        }
    }

    // Check to see if input looks like a Unix timestamp (i.e. can it be parsed to an int?)
    let timestamp = match input {
        Value::Int { val, .. } => Ok(*val),
        Value::String { val, .. } => val.parse::<i64>(),
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

    if dateformat.is_none() {
        if let Ok(ts) = timestamp {
            return match timezone {
                // note all these `.timestamp_nanos()` could overflow if we didn't check range in `<date> | into int`.

                // default to UTC
                None => Value::date(Utc.timestamp_nanos(ts).into(), head),
                Some(Spanned { item, span }) => match item {
                    Zone::Utc => {
                        let dt = Utc.timestamp_nanos(ts);
                        Value::date(dt.into(), *span)
                    }
                    Zone::Local => {
                        let dt = Local.timestamp_nanos(ts);
                        Value::date(dt.into(), *span)
                    }
                    Zone::East(i) => match FixedOffset::east_opt((*i as i32) * HOUR) {
                        Some(eastoffset) => {
                            let dt = eastoffset.timestamp_nanos(ts);
                            Value::date(dt, *span)
                        }
                        None => Value::error(
                            ShellError::DatetimeParseError {
                                msg: input.to_abbreviated_string(&nu_protocol::Config::default()),
                                span: *span,
                            },
                            *span,
                        ),
                    },
                    Zone::West(i) => match FixedOffset::west_opt((*i as i32) * HOUR) {
                        Some(westoffset) => {
                            let dt = westoffset.timestamp_nanos(ts);
                            Value::date(dt, *span)
                        }
                        None => Value::error(
                            ShellError::DatetimeParseError {
                                msg: input.to_abbreviated_string(&nu_protocol::Config::default()),
                                span: *span,
                            },
                            *span,
                        ),
                    },
                    Zone::Error => Value::error(
                        // This is an argument error, not an input error
                        ShellError::TypeMismatch {
                            err_message: "Invalid timezone or offset".to_string(),
                            span: *span,
                        },
                        *span,
                    ),
                },
            };
        };
    }

    // If input is not a timestamp, try parsing it as a string
    let span = input.span();

    let parse_as_string = |val: &str| {
        match dateformat {
            Some(dt_format) => match DateTime::parse_from_str(val, &dt_format.0) {
                Ok(dt) => {
                    match timezone {
                        None => {
                            Value::date ( dt, head )
                        },
                        Some(Spanned { item, span }) => match item {
                            Zone::Utc => {
                                Value::date ( dt, head )
                            }
                            Zone::Local => {
                                Value::date(dt.with_timezone(&Local).into(), *span)
                            }
                            Zone::East(i) => match FixedOffset::east_opt((*i as i32) * HOUR) {
                                Some(eastoffset) => {
                                    Value::date(dt.with_timezone(&eastoffset), *span)
                                }
                                None => Value::error(
                                    ShellError::DatetimeParseError {
                                        msg: input.to_abbreviated_string(&nu_protocol::Config::default()),
                                        span: *span,
                                    },
                                    *span,
                                ),
                            },
                            Zone::West(i) => match FixedOffset::west_opt((*i as i32) * HOUR) {
                                Some(westoffset) => {
                                    Value::date(dt.with_timezone(&westoffset), *span)
                                }
                                None => Value::error(
                                    ShellError::DatetimeParseError {
                                        msg: input.to_abbreviated_string(&nu_protocol::Config::default()),
                                        span: *span,
                                    },
                                    *span,
                                ),
                            },
                            Zone::Error => Value::error(
                                // This is an argument error, not an input error
                                ShellError::TypeMismatch {
                                    err_message: "Invalid timezone or offset".to_string(),
                                    span: *span,
                                },
                                *span,
                            ),
                        },
                    }
                },
                Err(reason) => {
                    match NaiveDateTime::parse_from_str(val, &dt_format.0) {
                        Ok(d) => {
                            let dt_fixed =
                                Local.from_local_datetime(&d).single().unwrap_or_default();

                            Value::date(dt_fixed.into(),head)
                        }
                        Err(_) => {
                            Value::error (
                                ShellError::CantConvert { to_type: format!("could not parse as datetime using format '{}'", dt_format.0), from_type: reason.to_string(), span: head, help: Some("you can use `into datetime` without a format string to enable flexible parsing".to_string()) },
                                head,
                            )
                        }
                    }
                }
            },

            // Tries to automatically parse the date
            // (i.e. without a format string)
            // and assumes the system's local timezone if none is specified
            None => match parse_date_from_string(val, span) {
                Ok(date) => Value::date (
                    date,
                    span,
                ),
                Err(err) => err,
            },
        }
    };

    match input {
        Value::String { val, .. } => parse_as_string(val),
        Value::Int { val, .. } => parse_as_string(&val.to_string()),

        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

fn list_human_readable_examples(span: Span) -> Value {
    let examples: Vec<String> = vec![
        "Today 18:30".into(),
        "2022-11-07 13:25:30".into(),
        "15:20 Friday".into(),
        "This Friday 17:00".into(),
        "13:25, Next Tuesday".into(),
        "Last Friday at 19:45".into(),
        "In 3 days".into(),
        "In 2 hours".into(),
        "10 hours and 5 minutes ago".into(),
        "1 years ago".into(),
        "A year ago".into(),
        "A month ago".into(),
        "A week ago".into(),
        "A day ago".into(),
        "An hour ago".into(),
        "A minute ago".into(),
        "A second ago".into(),
        "Now".into(),
    ];

    let records = examples
        .iter()
        .map(|s| {
            Value::record(
                record! {
                    "parseable human datetime examples" => Value::test_string(s.to_string()),
                    "result" => action(&Value::test_string(s.to_string()), &Arguments { zone_options: None, format_options: None, cell_paths: None }, span)
                },
                span,
            )
        })
        .collect::<Vec<Value>>();

    Value::list(records, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{action, DatetimeFormat, IntoDatetime, Zone};
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoDatetime {})
    }

    #[test]
    fn takes_a_date_format_with_timezone() {
        let date_str = Value::test_string("16.11.1984 8:00 am +0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("16.11.1984 8:00 am +0000", "%d.%m.%Y %H:%M %P %z").unwrap(),
            Span::test_data(),
        );
        assert_eq!(actual, expected)
    }

    #[test]
    #[ignore]
    fn takes_a_date_format_without_timezone() {
        // Ignoring this test for now because we changed the human-date-parser to use
        // the users timezone instead of UTC. We may continue to tweak this behavior.
        // Another hacky solution is to set the timezone to UTC in the test, which works
        // on MacOS and Linux but hasn't been tested on Windows. Plus it kind of defeats
        // the purpose of a "without_timezone" test.
        // std::env::set_var("TZ", "UTC");
        let date_str = Value::test_string("16.11.1984 8:00 am");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P".to_string()));
        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            Local
                .from_local_datetime(
                    &NaiveDateTime::parse_from_str("16.11.1984 8:00 am", "%d.%m.%Y %H:%M %P")
                        .unwrap(),
                )
                .unwrap()
                .with_timezone(Local::now().offset()),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = Value::test_string("2020-08-04T16:39:18+00:00");
        let args = Arguments {
            zone_options: None,
            format_options: None,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("2020-08-04T16:39:18+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(),
            Span::test_data(),
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset() {
        let date_str = Value::test_string("1614434140000000000");
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset_as_int() {
        let date_int = Value::test_int(1_614_434_140_000_000_000);
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };
        let actual = action(&date_int, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_int_with_formatstring() {
        let date_int = Value::test_int(1_614_434_140);
        let fmt_options = Some(DatetimeFormat("%s".to_string()));
        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_int, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset_as_int_with_formatting() {
        let date_int = Value::test_int(1_614_434_140);
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let fmt_options = Some(DatetimeFormat("%s".to_string()));
        let args = Arguments {
            zone_options: timezone_option,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_int, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset_as_int_with_local_timezone() {
        let date_int = Value::test_int(1_614_434_140);
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let fmt_options = Some(DatetimeFormat("%s".to_string()));
        let args = Arguments {
            zone_options: timezone_option,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_int, &args, Span::test_data());
        let expected = Value::date(
            Utc.timestamp_opt(1_614_434_140, 0).unwrap().into(),
            Span::test_data(),
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp() {
        let date_str = Value::test_string("1614434140000000000");
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            Local.timestamp_opt(1_614_434_140, 0).unwrap().into(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_datetime() {
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };
        let expected = Value::date(
            Local.timestamp_opt(1_614_434_140, 0).unwrap().into(),
            Span::test_data(),
        );
        let actual = action(&expected, &args, Span::test_data());

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_without_timezone() {
        let date_str = Value::test_string("1614434140000000000");
        let args = Arguments {
            zone_options: None,
            format_options: None,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());

        let expected = Value::date(
            Utc.timestamp_opt(1_614_434_140, 0).unwrap().into(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = Value::test_string("16.11.1984 8:00 am Oops0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }
}
