use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{
    DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone,
    Timelike, Utc,
};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

const HOUR: i32 = 60 * 60;
const ALLOWED_COLUMNS: [&str; 10] = [
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "millisecond",
    "microsecond",
    "nanosecond",
    "timezone",
];

#[derive(Clone, Debug)]
struct Arguments {
    zone_options: Option<Spanned<Zone>>,
    format_options: Option<Spanned<DatetimeFormat>>,
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
    const OPTIONS: &[&str] = &["utc", "local"];
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
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Date)),
                ),
                (Type::table(), Type::table()),
                (Type::Nothing, Type::table()),
                (Type::record(), Type::record()),
                (Type::record(), Type::Date),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                // only applicable for --list flag
                (Type::Any, Type::table()),
            ])
            .allow_variants_without_examples(true)
            .param(
                Flag::new("timezone")
                    .short('z')
                    .arg(SyntaxShape::String)
                    .desc(
                        "Specify timezone if the input is a Unix timestamp. Valid options: 'UTC' \
                         ('u') or 'LOCAL' ('l')",
                    )
                    .completion(Completion::new_list(Zone::OPTIONS)),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify timezone by offset from UTC if the input is a Unix timestamp, like '+8', \
                 '-4'",
                Some('o'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify expected format of INPUT string to parse to datetime. Use --list to see \
                 options",
                Some('f'),
            )
            .switch(
                "list",
                "Show all possible variables for use in --format flag",
                Some('l'),
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
                .get_flag::<Spanned<String>>(engine_state, stack, "format")?
                .as_ref()
                .map(|fmt| Spanned {
                    item: DatetimeFormat(fmt.item.to_string()),
                    span: fmt.span,
                });

            let args = Arguments {
                zone_options,
                format_options,
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

    fn examples(&self) -> Vec<Example<'_>> {
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
                description: "Convert non-standard timestamp string, with timezone offset, to \
                              datetime using a custom format",
                example: "'20210227_135540+0000' | into datetime --format '%Y%m%d_%H%M%S%z'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Convert non-standard timestamp string, without timezone offset, to \
                              datetime with custom formatting",
                example: "'16.11.1984 8:00 am' | into datetime --format '%d.%m.%Y %H:%M %P'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: Some(Value::date(
                    Local
                        .from_local_datetime(
                            &NaiveDateTime::parse_from_str(
                                "16.11.1984 8:00 am",
                                "%d.%m.%Y %H:%M %P",
                            )
                            .expect("date calculation should not fail in test"),
                        )
                        .unwrap()
                        .with_timezone(Local::now().offset()),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Convert nanosecond-precision unix timestamp to a datetime with \
                              offset from UTC",
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
                description: "Using a record as input",
                example: "{year: 2025, month: 3, day: 30, hour: 12, minute: 15, second: 59, \
                          timezone: '+02:00'} | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1743329759_000000000),
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
        ]
    }
}

#[derive(Clone, Debug)]
struct DatetimeFormat(String);

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let timezone = &args.zone_options;
    let dateformat = &args.format_options;

    // noop if the input is already a datetime
    if let Value::Date { .. } = input {
        return input.clone();
    }

    if let Value::Record { val: record, .. } = input {
        if let Some(tz) = timezone {
            return Value::error(
                ShellError::IncompatibleParameters {
                    left_message: "got a record as input".into(),
                    left_span: head,
                    right_message: "the timezone should be included in the record".into(),
                    right_span: tz.span,
                },
                head,
            );
        }

        if let Some(dt) = dateformat {
            return Value::error(
                ShellError::IncompatibleParameters {
                    left_message: "got a record as input".into(),
                    left_span: head,
                    right_message: "cannot be used with records".into(),
                    right_span: dt.span,
                },
                head,
            );
        }

        let span = input.span();
        return merge_record(record, head, span).unwrap_or_else(|err| Value::error(err, span));
    }

    // Let's try dtparse first
    if matches!(input, Value::String { .. }) && dateformat.is_none() {
        let span = input.span();
        if let Ok(input_val) = input.coerce_str()
            && let Ok(date) = parse_date_from_string(&input_val, span)
        {
            return Value::date(date, span);
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

    if dateformat.is_none()
        && let Ok(ts) = timestamp
    {
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

    // If input is not a timestamp, try parsing it as a string
    let span = input.span();

    let parse_as_string = |val: &str| {
        match dateformat {
            Some(dt_format) => {
                // Handle custom format specifiers for compact formats
                let format_str = dt_format
                    .item
                    .0
                    .replace("%J", "%Y%m%d") // %J for joined date (YYYYMMDD)
                    .replace("%Q", "%H%M%S"); // %Q for sequential time (HHMMSS)
                match DateTime::parse_from_str(val, &format_str) {
                    Ok(dt) => match timezone {
                        None => Value::date(dt, head),
                        Some(Spanned { item, span }) => match item {
                            Zone::Utc => Value::date(dt, head),
                            Zone::Local => Value::date(dt.with_timezone(&Local).into(), *span),
                            Zone::East(i) => match FixedOffset::east_opt((*i as i32) * HOUR) {
                                Some(eastoffset) => {
                                    Value::date(dt.with_timezone(&eastoffset), *span)
                                }
                                None => Value::error(
                                    ShellError::DatetimeParseError {
                                        msg: input
                                            .to_abbreviated_string(&nu_protocol::Config::default()),
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
                                        msg: input
                                            .to_abbreviated_string(&nu_protocol::Config::default()),
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
                    },
                    Err(reason) => parse_with_format(val, &format_str, head).unwrap_or_else(|_| {
                        Value::error(
                            ShellError::CantConvert {
                                to_type: format!(
                                    "could not parse as datetime using format '{}'",
                                    dt_format.item.0
                                ),
                                from_type: reason.to_string(),
                                span: head,
                                help: Some(
                                    "you can use `into datetime` without a format string to \
                                         enable flexible parsing"
                                        .to_string(),
                                ),
                            },
                            head,
                        )
                    }),
                }
            }

            // Tries to automatically parse the date
            // (i.e. without a format string)
            // and assumes the system's local timezone if none is specified
            None => match parse_date_from_string(val, span) {
                Ok(date) => Value::date(date, span),
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

fn merge_record(record: &Record, head: Span, span: Span) -> Result<Value, ShellError> {
    if let Some(invalid_col) = record
        .columns()
        .find(|key| !ALLOWED_COLUMNS.contains(&key.as_str()))
    {
        let allowed_cols = ALLOWED_COLUMNS.join(", ");
        return Err(ShellError::UnsupportedInput {
            msg: format!(
                "Column '{invalid_col}' is not valid for a structured datetime. Allowed columns \
                 are: {allowed_cols}"
            ),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        });
    };

    // Empty fields are filled in a specific way: the time units bigger than the biggest provided fields are assumed to be current and smaller ones are zeroed.
    // And local timezone is used if not provided.
    #[derive(Debug)]
    enum RecordColumnDefault {
        Now,
        Zero,
    }
    let mut record_column_default = RecordColumnDefault::Now;

    let now = Local::now();
    let mut now_nanosecond = now.nanosecond();
    let now_millisecond = now_nanosecond / 1_000_000;
    now_nanosecond %= 1_000_000;
    let now_microsecond = now_nanosecond / 1_000;
    now_nanosecond %= 1_000;

    let year: i32 = match record.get("year") {
        Some(val) => {
            record_column_default = RecordColumnDefault::Zero;
            match val {
                Value::Int { val, .. } => *val as i32,
                other => {
                    return Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "int".to_string(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: head,
                        src_span: other.span(),
                    });
                }
            }
        }
        None => now.year(),
    };
    let month = match record.get("month") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("month", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now.month(),
            RecordColumnDefault::Zero => 1,
        },
    };
    let day = match record.get("day") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("day", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now.day(),
            RecordColumnDefault::Zero => 1,
        },
    };
    let hour = match record.get("hour") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("hour", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now.hour(),
            RecordColumnDefault::Zero => 0,
        },
    };
    let minute = match record.get("minute") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("minute", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now.minute(),
            RecordColumnDefault::Zero => 0,
        },
    };
    let second = match record.get("second") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("second", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now.second(),
            RecordColumnDefault::Zero => 0,
        },
    };
    let millisecond = match record.get("millisecond") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("millisecond", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now_millisecond,
            RecordColumnDefault::Zero => 0,
        },
    };
    let microsecond = match record.get("microsecond") {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32("microsecond", col_val, &head, &span)?
        }
        None => match record_column_default {
            RecordColumnDefault::Now => now_microsecond,
            RecordColumnDefault::Zero => 0,
        },
    };

    let nanosecond = match record.get("nanosecond") {
        Some(col_val) => parse_value_from_record_as_u32("nanosecond", col_val, &head, &span)?,
        None => match record_column_default {
            RecordColumnDefault::Now => now_nanosecond,
            RecordColumnDefault::Zero => 0,
        },
    };

    let offset: FixedOffset = match record.get("timezone") {
        Some(timezone) => parse_timezone_from_record(timezone, &head, &timezone.span())?,
        None => now.offset().to_owned(),
    };

    let total_nanoseconds = nanosecond + microsecond * 1_000 + millisecond * 1_000_000;

    let date = match NaiveDate::from_ymd_opt(year, month, day) {
        Some(d) => d,
        None => {
            return Err(ShellError::IncorrectValue {
                msg: "one of more values are incorrect and do not represent valid date".to_string(),
                val_span: head,
                call_span: span,
            });
        }
    };
    let time = match NaiveTime::from_hms_nano_opt(hour, minute, second, total_nanoseconds) {
        Some(t) => t,
        None => {
            return Err(ShellError::IncorrectValue {
                msg: "one of more values are incorrect and do not represent valid time".to_string(),
                val_span: head,
                call_span: span,
            });
        }
    };
    let date_time = NaiveDateTime::new(date, time);

    let date_time_fixed = match offset.from_local_datetime(&date_time).single() {
        Some(d) => d,
        None => {
            return Err(ShellError::IncorrectValue {
                msg: "Ambiguous or invalid timezone conversion".to_string(),
                val_span: head,
                call_span: span,
            });
        }
    };
    Ok(Value::date(date_time_fixed, span))
}

fn parse_value_from_record_as_u32(
    col: &str,
    col_val: &Value,
    head: &Span,
    span: &Span,
) -> Result<u32, ShellError> {
    let value: u32 = match col_val {
        Value::Int { val, .. } => {
            if *val < 0 || *val > u32::MAX as i64 {
                return Err(ShellError::IncorrectValue {
                    msg: format!("incorrect value for {col}"),
                    val_span: *head,
                    call_span: *span,
                });
            }
            *val as u32
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

fn parse_timezone_from_record(
    timezone: &Value,
    head: &Span,
    span: &Span,
) -> Result<FixedOffset, ShellError> {
    match timezone {
        Value::String { val, .. } => {
            let offset: FixedOffset = match val.parse() {
                Ok(offset) => offset,
                Err(_) => {
                    return Err(ShellError::IncorrectValue {
                        msg: "invalid timezone".to_string(),
                        val_span: *span,
                        call_span: *head,
                    });
                }
            };
            Ok(offset)
        }
        other => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "string".to_string(),
            wrong_type: other.get_type().to_string(),
            dst_span: *head,
            src_span: other.span(),
        }),
    }
}

fn parse_with_format(val: &str, fmt: &str, head: Span) -> Result<Value, ()> {
    // try parsing at date + time
    if let Ok(dt) = NaiveDateTime::parse_from_str(val, fmt) {
        let dt_native = Local.from_local_datetime(&dt).single().unwrap_or_default();
        return Ok(Value::date(dt_native.into(), head));
    }

    // try parsing at date only
    if let Ok(date) = NaiveDate::parse_from_str(val, fmt)
        && let Some(dt) = date.and_hms_opt(0, 0, 0)
    {
        let dt_native = Local.from_local_datetime(&dt).single().unwrap_or_default();
        return Ok(Value::date(dt_native.into(), head));
    }

    // try parsing at time only
    if let Ok(time) = NaiveTime::parse_from_str(val, fmt) {
        let now = Local::now().naive_local().date();
        let dt_native = Local
            .from_local_datetime(&now.and_time(time))
            .single()
            .unwrap_or_default();
        return Ok(Value::date(dt_native.into(), head));
    }

    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{DatetimeFormat, IntoDatetime, Zone, action};
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoDatetime {})
    }

    #[test]
    fn takes_a_date_format_with_timezone() {
        let date_str = Value::test_string("16.11.1984 8:00 am +0000");
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()),
            span: Span::test_data(),
        });
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
    fn takes_a_date_format_without_timezone() {
        let date_str = Value::test_string("16.11.1984 8:00 am");
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%d.%m.%Y %H:%M %P".to_string()),
            span: Span::test_data(),
        });
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
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%s".to_string()),
            span: Span::test_data(),
        });
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
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%s".to_string()),
            span: Span::test_data(),
        });
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
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%s".to_string()),
            span: Span::test_data(),
        });
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
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()),
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }
}
