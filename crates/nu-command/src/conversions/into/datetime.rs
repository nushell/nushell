use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{DateTime, FixedOffset, Local, NaiveTime, TimeZone, Utc};
use human_date_parser::{from_human_time, ParseResult};
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;

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
    fn from_string(s: String) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "utc" | "u" => Self::Utc,
            "local" | "l" => Self::Local,
            _ => Self::Error,
        }
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into datetime"
    }

    fn signature(&self) -> Signature {
        Signature::build("into datetime")
        .input_output_types(vec![
            (Type::Int, Type::Date),
            (Type::String, Type::Date),
            (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Date))),
            (Type::table(), Type::table()),
            (Type::record(), Type::record()),
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
                        item: Zone::from_string(zone.item.clone()),
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
            operate(action, args, input, call.head, engine_state.ctrlc.clone())
        }
    }

    fn usage(&self) -> &str {
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
                description: "Convert any standard timestamp string to datetime",
                example: "'27.02.2021 1:55 pm +0000' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434100_000000000),
            },
            Example {
                description: "Convert any standard timestamp string to datetime",
                example: "'2021-02-27T13:55:40.2246+00:00' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_224600000),
            },
            Example {
                description:
                    "Convert non-standard timestamp string to datetime using a custom format",
                example: "'20210227_135540+0000' | into datetime --format '%Y%m%d_%H%M%S%z'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
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
                example: "1614434140 * 1_000_000_000 | into datetime",
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

#[derive(Clone)]
struct DatetimeFormat(String);

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let timezone = &args.zone_options;
    let dateformat = &args.format_options;

    // Let's try dtparse first
    if matches!(input, Value::String { .. }) && dateformat.is_none() {
        let span = input.span();
        if let Ok(input_val) = input.coerce_str() {
            match parse_date_from_string(&input_val, span) {
                Ok(date) => return Value::date(date, span),
                Err(_) => {
                    if let Ok(date) = from_human_time(&input_val) {
                        match date {
                            ParseResult::Date(date) => {
                                let time = NaiveTime::from_hms_opt(0, 0, 0).expect("valid time");
                                let combined = date.and_time(time);
                                let dt_fixed = DateTime::from_naive_utc_and_offset(
                                    combined,
                                    *Local::now().offset(),
                                );
                                return Value::date(dt_fixed, span);
                            }
                            ParseResult::DateTime(date) => {
                                return Value::date(date.fixed_offset(), span)
                            }
                            ParseResult::Time(time) => {
                                let date = Local::now().date_naive();
                                let combined = date.and_time(time);
                                let dt_fixed = DateTime::from_naive_utc_and_offset(
                                    combined,
                                    *Local::now().offset(),
                                );
                                return Value::date(dt_fixed, span);
                            }
                        }
                    }
                }
            };
        }
    }
    const HOUR: i32 = 60 * 60;

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
    match input {
        Value::String { val, .. } => {
            match dateformat {
                Some(dt) => match DateTime::parse_from_str(val, &dt.0) {
                    Ok(d) => Value::date ( d, head ),
                    Err(reason) => {
                        Value::error (
                            ShellError::CantConvert { to_type: format!("could not parse as datetime using format '{}'", dt.0), from_type: reason.to_string(), span: head, help: Some("you can use `into datetime` without a format string to enable flexible parsing".to_string()) },
                            head,
                        )
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
        }
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
    use super::{action, DatetimeFormat, SubCommand, Zone};
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn takes_a_date_format() {
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
            Local.timestamp_opt(1614434140, 0).unwrap().into(),
            Span::test_data(),
        );

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
            Utc.timestamp_opt(1614434140, 0).unwrap().into(),
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
