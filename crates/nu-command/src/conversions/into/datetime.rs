use crate::{
    TIME_ZONE_NAMES, generate_date_formats_list, generate_strftime_list, parse_date_from_string,
};
use jiff::{
    Timestamp, civil,
    tz::{Disambiguation, Offset, TimeZone},
};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::ParsedDate;
use std::{ops::Deref, sync::LazyLock};

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

/// The time zone `--timezone` or `--offset` names.
#[derive(Clone, Debug)]
enum Zone {
    Utc,
    Local,
    East(u8),
    West(u8),
    Tz(TimeZone),
    Error, // we want Nushell to cast it instead of Rust
}

static ZONE_OPTIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let builtin = ["LOCAL", "UTC"].into_iter();
    let timezones = TIME_ZONE_NAMES
        .iter()
        .copied()
        .filter(|tz| !matches!(tz.to_uppercase().as_str(), "LOCAL" | "UTC")); // deduplicate
    Vec::from_iter(builtin.chain(timezones))
});

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
            tz if let Ok(tz) = jiff::tz::db().get(tz) => Self::Tz(tz),
            _ => Self::Error,
        }
    }

    /// The jiff time zone, or `None` for [`Zone::Error`].
    fn time_zone(&self) -> Option<TimeZone> {
        let fixed = |seconds: i32| Offset::from_seconds(seconds).ok().map(TimeZone::fixed);
        match self {
            Zone::Utc => Some(TimeZone::UTC),
            Zone::Local => Some(TimeZone::system()),
            Zone::East(hours) => fixed(i32::from(*hours) * HOUR),
            Zone::West(hours) => fixed(-i32::from(*hours) * HOUR),
            Zone::Tz(time_zone) => Some(time_zone.clone()),
            Zone::Error => None,
        }
    }
}

/// `timestamp` shown at `time_zone`'s offset for that instant.
fn in_time_zone(timestamp: Timestamp, time_zone: &TimeZone) -> nu_protocol::DateTime {
    nu_protocol::DateTime::new(timestamp, time_zone.to_offset(timestamp))
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
                (Type::Duration, Type::Date),
                (Type::String, Type::Date),
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::Date)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Date)),
                ),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Date)),
                ),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::Date)),
                ),
                (Type::table(), Type::table()),
                (Type::Nothing, Type::table()),
                (Type::record(), Type::record()),
                (Type::record(), Type::Date),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                // only applicable for --list and --formats flags
                (Type::Any, Type::table()),
            ])
            .allow_variants_without_examples(true)
            .param(
                Flag::new("timezone")
                    .short('z')
                    .arg(SyntaxShape::String)
                    .desc(
                        "Specify timezone to interpret timestamps and formatted datetime input. Valid options: 'UTC' ('u'), 'LOCAL' ('l') or any of `date list-timezone`.",
                    )
                    .completion(Completion::new_list(ZONE_OPTIONS.deref())),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify timezone by offset from UTC to interpret timestamps and formatted datetime input, like '+8', '-4'.",
                Some('o'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify expected format of INPUT string to parse to datetime. Use --list to see options, or --formats for the formats read without one.",
                Some('f'),
            )
            .switch(
                "list",
                "Show all possible variables for use in --format flag.",
                Some('l'),
            )
            .switch(
                "formats",
                "Show the formats read without --format, with an example of each.",
                None,
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
        } else if call.has_flag(engine_state, stack, "formats")? {
            Ok(generate_date_formats_list(call.head).into_pipeline_data())
        } else {
            let cell_paths = call.rest(engine_state, stack, 0)?;
            let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

            let zone_options = {
                // if zone-offset is specified, then zone will be neglected
                let offset = call.get_flag::<Spanned<i64>>(engine_state, stack, "offset")?;
                let timezone = call.get_flag::<Spanned<String>>(engine_state, stack, "timezone")?;

                offset
                    .map(|offset| offset.map(Zone::new))
                    .or_else(|| timezone.map(|tz| tz.as_deref().map(Zone::from_string)))
            };

            let format_options = call
                .get_flag::<Spanned<String>>(engine_state, stack, "format")?
                .map(|fmt| fmt.map(DatetimeFormat));

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

    fn extra_description(&self) -> &str {
        "Without --format, a string is a date, a time, or a date with a time after T, a space or \
         a comma, and an offset or time zone can follow the time. Parts left out come from today, \
         and a time without an offset is local time. An int, or a string of digits, is \
         nanoseconds since 1970-01-01 UTC. Run `into datetime --formats` to list each format with \
         an example."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "timezone", "UTC"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let example_result_1 = |nanos: i64| {
            nu_protocol::DateTime::from_unix_nanos(nanos.into(), Offset::UTC).map(Value::test_date)
        };
        vec![
            Example {
                description: "Convert timestamp string to datetime with timezone offset.",
                example: "'27.02.2021 1:55 pm +0000' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434100_000000000),
            },
            Example {
                description: "Convert standard timestamp string to datetime with timezone offset.",
                example: "'2021-02-27T13:55:40.2246+00:00' | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_224600000),
            },
            Example {
                description: "Convert a date written with a month name, an ordinal and a 12-hour \
                              time.",
                example: "'April 18th, 2025 4:04 PM +05:30' | into datetime",
                result: nu_protocol::DateTime::parse_rfc3339("2025-04-18T16:04:00+05:30")
                    .ok()
                    .map(Value::test_date),
            },
            Example {
                description: "Convert a date-time in an IANA time zone.",
                example: "'2025-04-18 04:04:32 America/New_York' | into datetime",
                result: nu_protocol::DateTime::parse_rfc3339("2025-04-18T04:04:32-04:00")
                    .ok()
                    .map(Value::test_date),
            },
            Example {
                description: "Convert a relative date.",
                example: "'tomorrow 10am' | into datetime",
                result: None,
            },
            Example {
                description: "List the formats read without --format, with an example of each.",
                example: "into datetime --formats",
                result: None,
            },
            Example {
                description: "Convert non-standard timestamp string, with timezone offset, to \
                              datetime using a custom format.",
                example: "'20210227_135540+0000' | into datetime --format '%Y%m%d_%H%M%S%z'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Convert non-standard timestamp string, without timezone offset, to \
                              datetime with custom formatting.",
                example: "'16.11.1984 8:00 am' | into datetime --format '%d.%m.%Y %H:%M %P'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: civil::date(1984, 11, 16)
                    .at(8, 0, 0, 0)
                    .to_zoned_datetime(TimeZone::system())
                    .ok()
                    .map(|date| Value::test_date(nu_protocol::DateTime::from_zoned(&date))),
            },
            Example {
                description: "Interpret a formatted datetime in the Europe/Berlin timezone.",
                example: "'2024-07-01 12:00' | into datetime --format '%Y-%m-%d %H:%M' --timezone Europe/Berlin",
                result: Some(Value::date(
                    nu_protocol::DateTime::parse_rfc3339("2024-07-01T12:00:00+02:00")
                        .expect("date calculation should not fail in test"),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Convert nanosecond-precision unix timestamp to a datetime with \
                              offset from UTC.",
                example: "1614434140123456789 | into datetime --offset -5",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_123456789),
            },
            Example {
                description: "Convert standard (seconds) unix timestamp to a UTC datetime.",
                example: "1614434140 | into datetime -f '%s'",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Convert unix milliseconds timestamp to a UTC datetime.",
                example: "1614434140123 | into duration --unit ms | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_123000000),
            },
            Example {
                description: "Using a datetime as input simply returns the value.",
                example: "2021-02-27T13:55:40 | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1614434140_000000000),
            },
            Example {
                description: "Using a record as input.",
                example: "{year: 2025, month: 3, day: 30, hour: 12, minute: 15, second: 59, \
                          timezone: '+02:00'} | into datetime",
                #[allow(clippy::inconsistent_digit_grouping)]
                result: example_result_1(1743329759_000000000),
            },
            Example {
                description: "Convert list of timestamps to datetimes.",
                example: r#"["2023-03-30 10:10:07 -05:00", "2023-05-05 13:43:49 -05:00", "2023-06-05 01:37:42 -05:00"] | into datetime"#,
                result: Some(Value::list(
                    vec![
                        Value::date(
                            nu_protocol::DateTime::parse_rfc3339("2023-03-30T10:10:07-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            nu_protocol::DateTime::parse_rfc3339("2023-05-05T13:43:49-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            nu_protocol::DateTime::parse_rfc3339("2023-06-05T01:37:42-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Passing a list of datetimes through is a no-op.",
                example: "[2023-03-30T10:10:07-05:00, 2023-05-05T13:43:49-05:00] | into datetime",
                result: Some(Value::list(
                    vec![
                        Value::date(
                            nu_protocol::DateTime::parse_rfc3339("2023-03-30T10:10:07-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            nu_protocol::DateTime::parse_rfc3339("2023-05-05T13:43:49-05:00")
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

    // Without a format, try reading the string as a date (RFC 3339, then the other layouts) first.
    // A string that isn't a date or an integer timestamp is an error here, not parsed again below.
    if matches!(input, Value::String { .. }) && dateformat.is_none() {
        let span = input.span();
        if let Ok(input_val) = input.coerce_str() {
            match parse_date_from_string(&input_val, span) {
                Ok(date) => return Value::date(date, span),
                Err(err) if input_val.parse::<i64>().is_err() => return err,
                Err(_) => {}
            }
        }
    }

    let unsupported_type_error = |val: &Value| {
        Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string, int, duration".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: head,
                src_span: val.span(),
            },
            head,
        )
    };

    // Check to see if input looks like a Unix timestamp (i.e. can it be parsed to an int?)
    let timestamp = match input {
        Value::Int { val, .. } => Ok(*val),
        Value::Duration { val, .. } => match dateformat {
            Some(dt) => {
                return Value::error(
                    ShellError::IncompatibleParameters {
                        left_message: "got a duration as input".into(),
                        left_span: head,
                        right_message: "cannot be used with durations".into(),
                        right_span: dt.span,
                    },
                    head,
                );
            }
            None => Ok(*val),
        },
        Value::String { val, .. } => val.parse::<i64>(),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => return input.clone(),
        other => {
            return unsupported_type_error(other);
        }
    };

    if dateformat.is_none()
        && let Ok(ts) = timestamp
    {
        // Every i64 of nanoseconds is a valid timestamp (1677 to 2262).
        let Ok(timestamp) = Timestamp::from_nanosecond(ts.into()) else {
            return datetime_parse_error_value(
                &input.to_abbreviated_string(&nu_protocol::Config::default()),
                head,
            );
        };
        return match timezone {
            // default to UTC
            None => Value::date(nu_protocol::DateTime::new(timestamp, Offset::UTC), head),
            Some(Spanned { item, span }) => match item.time_zone() {
                Some(time_zone) => Value::date(in_time_zone(timestamp, &time_zone), *span),
                None => invalid_timezone_value(*span),
            },
        };
    };

    // If input is not a timestamp, try parsing it as a string
    let span = input.span();

    let parse_as_string = |val: &str| {
        match dateformat {
            Some(dt_format) => {
                // `%J` and `%Q` are nushell's joined date (YYYYMMDD) and time (HHMMSS).
                let format = dt_format
                    .item
                    .0
                    .replace("%J", "%Y%m%d")
                    .replace("%Q", "%H%M%S");
                let parsed = ParsedDate::parse(val, &format);
                match parsed
                    .as_ref()
                    .map_err(|err| *err)
                    .and_then(ParsedDate::to_datetime)
                {
                    Ok(date) => match timezone {
                        None => Value::date(date, head),
                        Some(Spanned { item, span }) => match item.time_zone() {
                            Some(time_zone) => Value::date(date.to_time_zone(&time_zone), *span),
                            None => invalid_timezone_value(*span),
                        },
                    },
                    // Without an offset, read a wall-clock date and time, a date or a time.
                    Err(reason) => match parsed.ok().as_ref().and_then(wall_clock) {
                        Some(wall) => {
                            interpret_wall_clock_datetime(wall, timezone.as_ref(), head, val)
                        }
                        None => Value::error(
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
                        ),
                    },
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
        other => unsupported_type_error(other),
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

    let now = nu_protocol::DateTime::now();
    let now_parts = now.parts();
    let mut now_nanosecond = now_parts.nanosecond as u32;
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
        None => now_parts.year,
    };
    let mut field = |name: &str, now: i8, zero: u32| match record.get(name) {
        Some(col_val) => {
            record_column_default = RecordColumnDefault::Zero;
            parse_value_from_record_as_u32(name, col_val, &head, &span)
        }
        None => Ok(match record_column_default {
            RecordColumnDefault::Now => now as u32,
            RecordColumnDefault::Zero => zero,
        }),
    };
    let month = field("month", now_parts.month, 1)?;
    let day = field("day", now_parts.day, 1)?;
    let hour = field("hour", now_parts.hour, 0)?;
    let minute = field("minute", now_parts.minute, 0)?;
    let second = field("second", now_parts.second, 0)?;
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

    // The offset in effect now, even for a date on the other side of a DST change.
    let offset = match record.get("timezone") {
        Some(timezone) => parse_timezone_from_record(timezone, &head, &timezone.span())?,
        None => now.offset(),
    };

    let invalid = |what: &str| ShellError::IncorrectValue {
        msg: format!("one of more values are incorrect and do not represent valid {what}"),
        val_span: head,
        call_span: span,
    };
    let small = |value: u32| i8::try_from(value).unwrap_or(i8::MAX);
    let (month, day, hour, minute, second) = (
        small(month),
        small(day),
        small(hour),
        small(minute),
        small(second),
    );
    // The date alone, at UTC midnight, tells a bad date from a bad time.
    nu_protocol::DateTime::from_parts(year, month, day, 0, 0, 0, 0, Offset::UTC)
        .ok_or_else(|| invalid("date"))?;
    let total_nanoseconds = millisecond
        .checked_mul(1_000_000)
        .zip(microsecond.checked_mul(1_000))
        .and_then(|(millis, micros)| millis.checked_add(micros)?.checked_add(nanosecond))
        .and_then(|nanos| i32::try_from(nanos).ok());
    // As before, a second's worth of extra nanoseconds on second 59 is a leap second.
    let nanos = total_nanoseconds
        .filter(|&nanos| {
            hour < 24
                && minute < 60
                && second < 60
                && (nanos < 1_000_000_000 || (nanos < 2_000_000_000 && second == 59))
        })
        .ok_or_else(|| invalid("time"))?;

    match nu_protocol::DateTime::from_parts(year, month, day, hour, minute, second, nanos, offset) {
        Some(date_time) => Ok(Value::date(date_time, span)),
        None => Err(ShellError::IncorrectValue {
            msg: "Ambiguous or invalid timezone conversion".to_string(),
            val_span: head,
            call_span: span,
        }),
    }
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
) -> Result<Offset, ShellError> {
    match timezone {
        Value::String { val, .. } => {
            let offset = match nu_protocol::parse_utc_offset(val) {
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

fn datetime_parse_error_value(val: &str, span: Span) -> Value {
    Value::error(
        ShellError::DatetimeParseError {
            msg: val.to_string(),
            span,
        },
        span,
    )
}

fn invalid_timezone_value(span: Span) -> Value {
    Value::error(
        // This is an argument error, not an input error
        ShellError::TypeMismatch {
            err_message: "Invalid timezone or offset".to_string(),
            span,
        },
        span,
    )
}

/// The wall-clock reading `parsed` names, held at UTC: a date and time, else midnight on a date,
/// else a time today.
fn wall_clock(parsed: &ParsedDate) -> Option<nu_protocol::DateTime> {
    parsed
        .to_naive_datetime()
        .or_else(|_| parsed.to_naive_date())
        .ok()
        .or_else(|| {
            let today = jiff::ZonedDateTime::now().date();
            let time = parsed.to_naive_time().ok()?;
            time.on(today.year().into(), today.month(), today.day())
        })
}

/// The instant the wall-clock `dt` names in `timezone`, or local time without one. A time the
/// zone skips or repeats at a DST change is an error.
fn interpret_wall_clock_datetime(
    dt: nu_protocol::DateTime,
    timezone: Option<&Spanned<Zone>>,
    head: Span,
    val: &str,
) -> Value {
    let (time_zone, span, local) = match timezone {
        None => (Some(TimeZone::system()), head, true),
        Some(Spanned { item, span }) => (item.time_zone(), *span, matches!(item, Zone::Local)),
    };
    let Some(time_zone) = time_zone else {
        return invalid_timezone_value(span);
    };
    // Local time is read as chrono's `Local` read it, taking a single reading only.
    let date = match dt.to_civil() {
        Some(civil) if local && !dt.parts().is_leap_second() => {
            match nu_protocol::DateTime::chrono_local_readings(civil, &time_zone)[..] {
                [date] => Some(date),
                _ => None,
            }
        }
        _ => dt.wall_clock_in(&time_zone, Disambiguation::Reject),
    };
    match date {
        Some(date) => Value::date(date, span),
        None => datetime_parse_error_value(val, span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{DatetimeFormat, IntoDatetime, Zone, action};
    use nu_protocol::{DateTime, Type::Error};

    /// The wall-clock `text` in the system time zone.
    fn local(text: &str) -> DateTime {
        let civil: civil::DateTime = text.parse().unwrap();
        DateTime::from_zoned(&civil.to_zoned_datetime(TimeZone::system()).unwrap())
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoDatetime)
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
            nu_protocol::DateTime::parse_rfc3339("1984-11-16T08:00:00+00:00").unwrap(),
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
        let expected = Value::date(local("1984-11-16T08:00:00"), Span::test_data());

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_a_date_format_without_timezone_with_utc_timezone() {
        let date_str = Value::test_string("2026-03-21_00:25");
        let timezone_option = Some(Spanned {
            item: Zone::Utc,
            span: Span::test_data(),
        });
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%F_%R".to_string()),
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());
        let expected = Value::date(
            DateTime::parse_rfc3339("2026-03-21T00:25:00Z").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_a_date_format_without_timezone_with_offset_timezone() {
        let date_str = Value::test_string("29 Aug 2025 19:30:07");
        let timezone_option = Some(Spanned {
            item: Zone::East(3),
            span: Span::test_data(),
        });
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%d %b %Y %H:%M:%S".to_string()),
            span: Span::test_data(),
        });
        let args = Arguments {
            zone_options: timezone_option,
            format_options: fmt_options,
            cell_paths: None,
        };
        let actual = action(&date_str, &args, Span::test_data());

        let expected = Value::date(
            DateTime::parse_rfc3339("2025-08-29T19:30:07+03:00").unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_a_date_format_without_timezone_applies_timezone_and_offset_differently() {
        let date_str = Value::test_string("2026-03-21_00:25");
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%F_%R".to_string()),
            span: Span::test_data(),
        });

        let utc_args = Arguments {
            zone_options: Some(Spanned {
                item: Zone::Utc,
                span: Span::test_data(),
            }),
            format_options: fmt_options.clone(),
            cell_paths: None,
        };
        let east_args = Arguments {
            zone_options: Some(Spanned {
                item: Zone::East(1),
                span: Span::test_data(),
            }),
            format_options: fmt_options,
            cell_paths: None,
        };

        let utc_actual = action(&date_str, &utc_args, Span::test_data());
        let east_actual = action(&date_str, &east_args, Span::test_data());

        assert_ne!(utc_actual, east_actual)
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
            nu_protocol::DateTime::parse_rfc3339("2020-08-04T16:39:18+00:00").unwrap(),
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
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T21:55:40+08:00").unwrap(),
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
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T21:55:40+08:00").unwrap(),
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
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T21:55:40+08:00").unwrap(),
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
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T21:55:40+08:00").unwrap(),
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
            DateTime::from_unix_seconds(1_614_434_140, 0, Offset::UTC).unwrap(),
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
            DateTime::from_timestamp_local(Timestamp::from_second(1_614_434_140).unwrap()),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_duration() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);

        let args = Arguments {
            zone_options: None,
            format_options: None,
            cell_paths: None,
        };

        let actual = action(&input_duration, &args, Span::test_data());
        let expected = Value::date(
            DateTime::from_unix_nanos(1_614_434_140_000_000_000, Offset::UTC).unwrap(),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_duration_with_local() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });

        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };

        let actual = action(&input_duration, &args, Span::test_data());
        let expected = Value::date(
            DateTime::from_timestamp_local(Timestamp::from_second(1_614_434_140).unwrap()),
            Span::test_data(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_duration_with_tz() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);
        let timezone_option = Some(Spanned {
            item: Zone::Tz(
                jiff::tz::db()
                    .get("Europe/Berlin")
                    .expect("Europe/Berlin is in the database"),
            ),
            span: Span::test_data(),
        });

        let args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };

        let actual = action(&input_duration, &args, Span::test_data());
        let expected = Value::test_date(
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T14:55:40+01:00").unwrap(),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn takes_duration_with_offset() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });

        let mut args = Arguments {
            zone_options: timezone_option,
            format_options: None,
            cell_paths: None,
        };

        let actual = action(&input_duration, &args, Span::test_data());
        let expected = Value::test_date(
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T21:55:40+08:00").unwrap(),
        );

        assert_eq!(actual, expected);

        args.zone_options
            .replace(Zone::West(8).into_spanned(Span::test_data()));

        let actual = action(&input_duration, &args, Span::test_data());

        let expected = Value::test_date(
            nu_protocol::DateTime::parse_rfc3339("2021-02-27T05:55:40-08:00").unwrap(),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn fails_with_duration_with_formatstring() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);
        let fmt_options = Some(Spanned {
            item: DatetimeFormat("%s".to_string()),
            span: Span::test_data(),
        });

        let args = Arguments {
            zone_options: None,
            format_options: fmt_options,
            cell_paths: None,
        };

        let actual = action(&input_duration, &args, Span::test_data());

        assert!(matches!(
            actual.unwrap_error(),
            Err(ShellError::IncompatibleParameters { .. })
        ));
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
            DateTime::from_timestamp_local(Timestamp::from_second(1_614_434_140).unwrap()),
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
            DateTime::from_unix_seconds(1_614_434_140, 0, Offset::UTC).unwrap(),
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
