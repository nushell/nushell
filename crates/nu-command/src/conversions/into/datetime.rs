//! `into datetime`: strings, ints, durations and records to dates.
//!
//! Each value takes two steps. [`convert`] reads it as a [`Reading`]: an instant when the input
//! has an offset or counts nanoseconds, or a wall-clock date and time when it has no offset.
//! [`Reading::place`] then makes the date. An instant is shown in the zone `--timezone` or
//! `--offset` names, and a wall-clock time is read in that zone, or in local time without one.
//!
//! Where each input is read:
//! - A string without `--format` goes to [`parse_date_from_string_in`] in `date/utils.rs`,
//!   which tries RFC 3339 and then the layouts in `date/formats.rs`. Those need the zone for
//!   today's date and for relative dates, so they place the date themselves. A string of digits
//!   is nanoseconds.
//! - A string or int with `--format` goes to [`read_with_format`], on nu-protocol's
//!   `ParsedDate`.
//! - A record goes to [`merge_record`]. It names its own offset, so it takes no flags.

use crate::{
    TIME_ZONE_NAMES, generate_date_formats_list, generate_strftime_list, parse_date_from_string_in,
    time_zone_named,
};
use jiff::{
    Timestamp, civil,
    tz::{Offset, TimeZone},
};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{DateParseError, DateTime, ParsedDate};
use std::{ops::Deref, sync::LazyLock};

/// The columns a record can have: the units of a date and time, biggest first, then its offset.
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
    /// The zone `--offset` or `--timezone` names.
    zone: Option<Spanned<TimeZone>>,
    /// The `--format` strings and ints are read with.
    format: Option<Spanned<String>>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

/// The zone `--offset` or `--timezone` names, or an error at the flag when it names none, before
/// any input is read. `--offset` is whole hours east of UTC, up to 12 either way, and takes
/// precedence. `--timezone` is `utc` (`u`), `local` (`l`) or an IANA name, in any case.
fn zone_flag(
    offset: Option<Spanned<i64>>,
    timezone: Option<Spanned<String>>,
) -> Result<Option<Spanned<TimeZone>>, ShellError> {
    let (zone, span) = match (offset, timezone) {
        (Some(Spanned { item: hours, span }), _) => {
            let zone = i8::try_from(hours)
                .ok()
                .filter(|hours| (-12..=12).contains(hours))
                .and_then(|hours| Offset::from_hours(hours).ok())
                .map(TimeZone::fixed);
            (zone, span)
        }
        (None, Some(Spanned { item: name, span })) => {
            let zone = match name.to_ascii_lowercase().as_str() {
                "utc" | "u" => Some(TimeZone::UTC),
                "local" | "l" => Some(TimeZone::system()),
                name => time_zone_named(name),
            };
            (zone, span)
        }
        (None, None) => return Ok(None),
    };
    match zone {
        Some(zone) => Ok(Some(zone.into_spanned(span))),
        None => Err(ShellError::TypeMismatch {
            err_message: "Invalid timezone or offset".to_string(),
            span,
        }),
    }
}

static ZONE_OPTIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let builtin = ["LOCAL", "UTC"].into_iter();
    let timezones = TIME_ZONE_NAMES
        .iter()
        .copied()
        .filter(|tz| !matches!(tz.to_uppercase().as_str(), "LOCAL" | "UTC")); // deduplicate
    Vec::from_iter(builtin.chain(timezones))
});

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
                        "Specify the timezone of the result: a date or time without an offset is read in it, and timestamps and strings with an offset are converted to it. A datetime input is returned unchanged. Valid options: 'UTC' ('u'), 'LOCAL' ('l') or any of `date list-timezone`.",
                    )
                    .completion(Completion::new_list(ZONE_OPTIONS.deref())),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify the timezone as hours from UTC, like '+8', '-4'. It works like --timezone and takes precedence over it.",
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

            let args = Arguments {
                zone: zone_flag(
                    call.get_flag(engine_state, stack, "offset")?,
                    call.get_flag(engine_state, stack, "timezone")?,
                )?,
                format: call.get_flag(engine_state, stack, "format")?,
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
         and a time without an offset is local time, or in the zone --timezone or --offset names. \
         An int, or a string of digits, is nanoseconds since 1970-01-01 UTC. Run \
         `into datetime --formats` to list each format with an example."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "timezone", "UTC"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let example_result_1 =
            |nanos: i64| DateTime::from_unix_nanos(nanos.into(), Offset::UTC).map(Value::test_date);
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
                result: DateTime::parse_rfc3339("2025-04-18T16:04:00+05:30")
                    .ok()
                    .map(Value::test_date),
            },
            Example {
                description: "Convert a date-time in an IANA time zone.",
                example: "'2025-04-18 04:04:32 America/New_York' | into datetime",
                result: DateTime::parse_rfc3339("2025-04-18T04:04:32-04:00")
                    .ok()
                    .map(Value::test_date),
            },
            Example {
                description: "Read a date-time without an offset as a time at UTC+8.",
                example: "'26-11-2026 05:00:00' | into datetime --offset 8",
                result: DateTime::parse_rfc3339("2026-11-26T05:00:00+08:00")
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
                result: DateTime::from_civil_local(civil::date(1984, 11, 16).at(8, 0, 0, 0))
                    .map(Value::test_date),
            },
            Example {
                description: "Interpret a formatted datetime in the Europe/Berlin timezone.",
                example: "'2024-07-01 12:00' | into datetime --format '%Y-%m-%d %H:%M' --timezone Europe/Berlin",
                result: Some(Value::date(
                    DateTime::parse_rfc3339("2024-07-01T12:00:00+02:00")
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
                            DateTime::parse_rfc3339("2023-03-30T10:10:07-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_rfc3339("2023-05-05T13:43:49-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_rfc3339("2023-06-05T01:37:42-05:00")
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
                            DateTime::parse_rfc3339("2023-03-30T10:10:07-05:00")
                                .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_rfc3339("2023-05-05T13:43:49-05:00")
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

/// Converts one value, for [`operate`].
fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    convert(input, args, head).unwrap_or_else(|error| error)
}

/// Reads `input` and places the reading in the flag's zone. A datetime or an error passes
/// through, and a record rejects the flags.
fn convert(input: &Value, args: &Arguments, head: Span) -> Result<Value, Value> {
    let span = input.span();
    let zone = args.zone.as_ref().map(|zone| &zone.item);
    let with_format = |text: &str, format: &Spanned<String>| {
        read_with_format(text, &format.item, zone)
            .map_err(|reason| format_error(&format.item, reason, head))
    };
    let reading = match (input, &args.format) {
        (Value::Date { .. } | Value::Error { .. }, _) => return Ok(input.clone()),
        (Value::Record { val, .. }, _) => return read_record(val, args, head, span),
        (Value::String { val, .. }, None) => read_text(val, zone, span)?,
        (Value::Int { val, .. } | Value::Duration { val, .. }, None) => Reading::Nanos(*val),
        (Value::String { val, .. }, Some(format)) => with_format(val, format)?,
        (Value::Int { val, .. }, Some(format)) => with_format(&val.to_string(), format)?,
        (Value::Duration { .. }, Some(format)) => {
            return Err(incompatible_flag(
                "got a duration as input",
                "cannot be used with durations",
                format.span,
                head,
            ));
        }
        (other, _) => return Err(unsupported_type(other, head)),
    };
    match reading.place(zone) {
        Some(date) => Ok(Value::date(date, span)),
        None => Err(datetime_parse_error_value(
            &input.to_abbreviated_string(&nu_protocol::Config::default()),
            head,
        )),
    }
}

/// What a value reads as, before [`Reading::place`] puts it in the flag's zone.
enum Reading {
    /// A point in time: the input had an offset, or its reader placed it in the zone already.
    Instant(DateTime),
    /// Nanoseconds since 1970-01-01 UTC: an int, a duration or a string of digits.
    Nanos(i64),
    /// A wall-clock date and time without an offset, held at UTC.
    WallClock(DateTime),
}

impl Reading {
    /// The date this reading names: an instant shown in `zone`, or a wall-clock time read in
    /// `zone`, or in local time without one. `None` when that date is out of range.
    fn place(self, zone: Option<&TimeZone>) -> Option<DateTime> {
        let instant = match self {
            Reading::Instant(date) => date,
            // Every i64 of nanoseconds is a valid timestamp (1677 to 2262).
            Reading::Nanos(nanos) => {
                DateTime::new(Timestamp::from_nanosecond(nanos.into()).ok()?, Offset::UTC)
            }
            // A time the clock skips at a DST change moves forward by the jump, and one it
            // repeats reads as the earlier, as `into datetime` reads them without `--format`.
            Reading::WallClock(wall) => {
                return match zone {
                    Some(zone) => wall.wall_clock_in(zone),
                    None => wall.wall_clock_in(&TimeZone::system()),
                };
            }
        };
        Some(match zone {
            Some(zone) => instant.to_time_zone(zone),
            None => instant,
        })
    }
}

/// A string without `--format`: a date in one of the formats `into datetime --formats` lists,
/// or else, if it's all digits, nanoseconds since 1970-01-01 UTC, like an int or a duration.
fn read_text(text: &str, zone: Option<&TimeZone>, span: Span) -> Result<Reading, Value> {
    match parse_date_from_string_in(text, zone, span) {
        Ok(date) => Ok(Reading::Instant(date)),
        Err(error) => text.parse().map(Reading::Nanos).map_err(|_| error),
    }
}

/// Reads `val` with `format`: an instant, or else a wall-clock date and time, midnight on a date
/// with no time of day, or a time of day today, taken in `zone` (local time without one). An
/// offset in the text places the smaller readings too, so `2021-03-04 +0530` with `%F %z` is
/// midnight at `+05:30`, and a time alone is today at that offset. A date that doesn't exist
/// (February 30) or a wrong weekday is an error. See `ParsedDate` for how jiff reads the rest.
fn read_with_format(
    val: &str,
    format: &str,
    zone: Option<&TimeZone>,
) -> Result<Reading, DateParseError> {
    let parsed = ParsedDate::parse(val, format)?;
    match parsed.to_datetime() {
        Err(DateParseError::NotEnough) => {}
        result => return result.map(Reading::Instant),
    }
    let offset = parsed.offset();
    let at_offset = |wall: DateTime| match offset {
        Some(offset) => wall
            .wall_clock_in(&TimeZone::fixed(offset))
            .map(Reading::Instant)
            .ok_or(DateParseError::OutOfRange),
        None => Ok(Reading::WallClock(wall)),
    };
    match parsed.to_wall_clock() {
        Err(DateParseError::NotEnough) => {}
        result => return at_offset(result?),
    }
    let today_zone = match (offset, zone) {
        (Some(offset), _) => TimeZone::fixed(offset),
        (None, Some(zone)) => zone.clone(),
        (None, None) => TimeZone::system(),
    };
    let today = Timestamp::now().to_zoned_datetime(today_zone).date();
    let wall = DateTime::from_civil(today.to_datetime(parsed.to_time()?), Offset::UTC)
        .ok_or(DateParseError::OutOfRange)?;
    at_offset(wall)
}

/// A record of date and time units. It names its own offset, so it takes neither flag.
fn read_record(record: &Record, args: &Arguments, head: Span, span: Span) -> Result<Value, Value> {
    if let Some(zone) = &args.zone {
        return Err(incompatible_flag(
            "got a record as input",
            "the timezone should be included in the record",
            zone.span,
            head,
        ));
    }
    if let Some(format) = &args.format {
        return Err(incompatible_flag(
            "got a record as input",
            "cannot be used with records",
            format.span,
            head,
        ));
    }
    merge_record(record, head, span).map_err(|error| Value::error(error, span))
}

/// The date a record of units names. A unit the record leaves out is taken from now while no
/// bigger unit is given, and is its smallest value (month and day 1, the rest 0) after one is,
/// so `{year: 2025}` is midnight on January 1st. A record without `timezone` is local time.
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

    let now = DateTime::now().parts();
    let now_nanos = now.nanosecond as u32;
    let unit = |name: &str, now: u32, smallest: u32| {
        // `timezone` comes after the units, so it never counts as a bigger one.
        let bigger_given = ALLOWED_COLUMNS
            .iter()
            .take_while(|column| **column != name)
            .any(|column| record.contains(column));
        match record.get(name) {
            Some(value) => parse_value_from_record_as_u32(name, value, &head, &span),
            None if bigger_given => Ok(smallest),
            None => Ok(now),
        }
    };
    let year = match record.get("year") {
        Some(Value::Int { val, .. }) => {
            i32::try_from(*val).map_err(|_| ShellError::IncorrectValue {
                msg: "incorrect value for year".to_string(),
                val_span: head,
                call_span: span,
            })?
        }
        Some(other) => {
            return Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "int".to_string(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            });
        }
        None => now.year,
    };
    let month = unit("month", now.month as u32, 1)?;
    let day = unit("day", now.day as u32, 1)?;
    let hour = unit("hour", now.hour as u32, 0)?;
    let minute = unit("minute", now.minute as u32, 0)?;
    let second = unit("second", now.second as u32, 0)?;
    let millisecond = unit("millisecond", now_nanos / 1_000_000, 0)?;
    let microsecond = unit("microsecond", now_nanos / 1_000 % 1_000, 0)?;
    let nanosecond = unit("nanosecond", now_nanos % 1_000, 0)?;

    let offset = match record.get("timezone") {
        Some(timezone) => Some(parse_timezone_from_record(
            timezone,
            &head,
            &timezone.span(),
        )?),
        None => None,
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
    DateTime::from_parts(year, month, day, 0, 0, 0, 0, Offset::UTC)
        .ok_or_else(|| invalid("date"))?;
    let total_nanoseconds = millisecond
        .checked_mul(1_000_000)
        .zip(microsecond.checked_mul(1_000))
        .and_then(|(millis, micros)| millis.checked_add(micros)?.checked_add(nanosecond))
        .and_then(|nanos| i32::try_from(nanos).ok());
    // As before, a second's worth of extra nanoseconds on second 59 is read, as chrono's leap
    // second, which is second 60; it reads as second 59, as a leap second reads everywhere.
    let nanos = total_nanoseconds
        .filter(|&nanos| {
            hour < 24
                && minute < 60
                && second < 60
                && (nanos < 1_000_000_000 || (nanos < 2_000_000_000 && second == 59))
        })
        .ok_or_else(|| invalid("time"))?;
    let (second, nanos) = if nanos >= 1_000_000_000 {
        (60, nanos - 1_000_000_000)
    } else {
        (second, nanos)
    };

    let reading = match offset {
        Some(offset) => DateTime::from_parts(year, month, day, hour, minute, second, nanos, offset)
            .map(Reading::Instant),
        None => DateTime::from_parts(year, month, day, hour, minute, second, nanos, Offset::UTC)
            .map(Reading::WallClock),
    };
    match reading.and_then(|reading| reading.place(None)) {
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

/// The error for a flag the input's type can't take.
fn incompatible_flag(input: &str, flag: &str, flag_span: Span, head: Span) -> Value {
    Value::error(
        ShellError::IncompatibleParameters {
            left_message: input.into(),
            left_span: head,
            right_message: flag.into(),
            right_span: flag_span,
        },
        head,
    )
}

/// The error for an input type `into datetime` doesn't convert.
fn unsupported_type(value: &Value, head: Span) -> Value {
    Value::error(
        ShellError::OnlySupportsThisInputType {
            exp_input_type: "string, int, duration".into(),
            wrong_type: value.get_type().to_string(),
            dst_span: head,
            src_span: value.span(),
        },
        head,
    )
}

/// The error for text that `--format` doesn't read.
fn format_error(format: &str, reason: DateParseError, head: Span) -> Value {
    Value::error(
        ShellError::CantConvert {
            to_type: format!("could not parse as datetime using format '{format}'"),
            from_type: reason.to_string(),
            span: head,
            help: Some(
                "you can use `into datetime` without a format string to enable flexible parsing"
                    .to_string(),
            ),
        },
        head,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Type::Error;

    /// `into datetime` on `input`, with the zone flag `zone` and `--format format`.
    fn into_datetime(input: Value, zone: Option<Spanned<TimeZone>>, format: Option<&str>) -> Value {
        let args = Arguments {
            zone,
            format: format.map(|format| format.to_string().into_spanned(Span::test_data())),
            cell_paths: None,
        };
        action(&input, &args, Span::test_data())
    }

    /// The zone `--offset hours` names, or its error.
    fn try_offset(hours: i64) -> Result<Option<Spanned<TimeZone>>, ShellError> {
        zone_flag(Some(hours.into_spanned(Span::test_data())), None)
    }

    /// The zone `--timezone name` names, or its error.
    fn try_timezone(name: &str) -> Result<Option<Spanned<TimeZone>>, ShellError> {
        zone_flag(None, Some(name.to_string().into_spanned(Span::test_data())))
    }

    /// The zone `--offset hours` names.
    fn offset(hours: i64) -> Option<Spanned<TimeZone>> {
        try_offset(hours).unwrap()
    }

    /// The zone `--timezone name` names.
    fn timezone(name: &str) -> Option<Spanned<TimeZone>> {
        try_timezone(name).unwrap()
    }

    /// The RFC 3339 date-time `text`.
    fn rfc3339(text: &str) -> Value {
        Value::test_date(DateTime::parse_rfc3339(text).unwrap())
    }

    /// The wall-clock `text` in the system time zone.
    fn local(text: &str) -> Value {
        let civil: civil::DateTime = text.parse().unwrap();
        Value::test_date(DateTime::from_civil_local(civil).unwrap())
    }

    /// 2021-02-27 13:55:40 UTC, in the system time zone.
    fn local_timestamp() -> Value {
        Value::test_date(DateTime::from_timestamp_local(
            Timestamp::from_second(1_614_434_140).unwrap(),
        ))
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoDatetime)
    }

    #[test]
    fn takes_a_date_format_with_timezone() {
        let actual = into_datetime(
            Value::test_string("16.11.1984 8:00 am +0000"),
            None,
            Some("%d.%m.%Y %H:%M %P %z"),
        );
        assert_eq!(actual, rfc3339("1984-11-16T08:00:00+00:00"))
    }

    #[test]
    fn takes_a_date_format_without_timezone() {
        let actual = into_datetime(
            Value::test_string("16.11.1984 8:00 am"),
            None,
            Some("%d.%m.%Y %H:%M %P"),
        );
        assert_eq!(actual, local("1984-11-16T08:00:00"))
    }

    #[test]
    fn takes_a_date_format_without_timezone_with_utc_timezone() {
        let actual = into_datetime(
            Value::test_string("2026-03-21_00:25"),
            timezone("UTC"),
            Some("%F_%R"),
        );
        assert_eq!(actual, rfc3339("2026-03-21T00:25:00Z"))
    }

    #[test]
    fn takes_a_date_format_without_timezone_with_offset_timezone() {
        let actual = into_datetime(
            Value::test_string("29 Aug 2025 19:30:07"),
            offset(3),
            Some("%d %b %Y %H:%M:%S"),
        );
        assert_eq!(actual, rfc3339("2025-08-29T19:30:07+03:00"))
    }

    #[test]
    fn takes_a_date_format_without_timezone_applies_timezone_and_offset_differently() {
        let date_str = Value::test_string("2026-03-21_00:25");
        let utc_actual = into_datetime(date_str.clone(), timezone("UTC"), Some("%F_%R"));
        let east_actual = into_datetime(date_str, offset(1), Some("%F_%R"));
        assert_ne!(utc_actual, east_actual)
    }

    #[test]
    fn takes_iso8601_date_format() {
        let actual = into_datetime(Value::test_string("2020-08-04T16:39:18+00:00"), None, None);
        assert_eq!(actual, rfc3339("2020-08-04T16:39:18+00:00"))
    }

    #[test]
    fn takes_timestamp_offset() {
        let actual = into_datetime(Value::test_string("1614434140000000000"), offset(8), None);
        assert_eq!(actual, rfc3339("2021-02-27T21:55:40+08:00"))
    }

    #[test]
    fn takes_timestamp_offset_as_int() {
        let actual = into_datetime(Value::test_int(1_614_434_140_000_000_000), offset(8), None);
        assert_eq!(actual, rfc3339("2021-02-27T21:55:40+08:00"))
    }

    #[test]
    fn takes_int_with_formatstring() {
        let actual = into_datetime(Value::test_int(1_614_434_140), None, Some("%s"));
        assert_eq!(actual, rfc3339("2021-02-27T21:55:40+08:00"))
    }

    #[test]
    fn takes_timestamp_offset_as_int_with_formatting() {
        let actual = into_datetime(Value::test_int(1_614_434_140), offset(8), Some("%s"));
        assert_eq!(actual, rfc3339("2021-02-27T21:55:40+08:00"))
    }

    #[test]
    fn takes_timestamp_offset_as_int_with_local_timezone() {
        let actual = into_datetime(
            Value::test_int(1_614_434_140),
            timezone("local"),
            Some("%s"),
        );
        assert_eq!(actual, local_timestamp())
    }

    #[test]
    fn takes_timestamp() {
        let actual = into_datetime(
            Value::test_string("1614434140000000000"),
            timezone("local"),
            None,
        );
        assert_eq!(actual, local_timestamp())
    }

    #[test]
    fn takes_duration() {
        let actual = into_datetime(Value::test_duration(1_614_434_140_000_000_000), None, None);
        assert_eq!(actual, rfc3339("2021-02-27T13:55:40Z"))
    }

    #[test]
    fn takes_duration_with_local() {
        let actual = into_datetime(
            Value::test_duration(1_614_434_140_000_000_000),
            timezone("local"),
            None,
        );
        assert_eq!(actual, local_timestamp())
    }

    #[test]
    fn takes_duration_with_tz() {
        let actual = into_datetime(
            Value::test_duration(1_614_434_140_000_000_000),
            timezone("Europe/Berlin"),
            None,
        );
        assert_eq!(actual, rfc3339("2021-02-27T14:55:40+01:00"));
    }

    #[test]
    fn takes_duration_with_offset() {
        let input_duration = Value::test_duration(1_614_434_140_000_000_000);
        let actual = into_datetime(input_duration.clone(), offset(8), None);
        assert_eq!(actual, rfc3339("2021-02-27T21:55:40+08:00"));

        let actual = into_datetime(input_duration, offset(-8), None);
        assert_eq!(actual, rfc3339("2021-02-27T05:55:40-08:00"));
    }

    #[test]
    fn fails_with_duration_with_formatstring() {
        let actual = into_datetime(
            Value::test_duration(1_614_434_140_000_000_000),
            None,
            Some("%s"),
        );
        assert!(matches!(
            actual.unwrap_error(),
            Err(ShellError::IncompatibleParameters { .. })
        ));
    }

    #[test]
    fn takes_datetime() {
        let expected = local_timestamp();
        let actual = into_datetime(expected.clone(), timezone("local"), None);
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_without_timezone() {
        let actual = into_datetime(Value::test_string("1614434140000000000"), None, None);
        assert_eq!(actual, rfc3339("2021-02-27T13:55:40Z"))
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let actual = into_datetime(
            Value::test_string("16.11.1984 8:00 am Oops0000"),
            None,
            Some("%d.%m.%Y %H:%M %P %z"),
        );
        assert_eq!(actual.get_type(), Error);
    }

    /// An offset past 12 hours either way is an error, down to the smallest int.
    #[test]
    fn rejects_offsets_past_12_hours() {
        for hours in [13, -13, 128, i64::MAX, i64::MIN] {
            assert!(
                matches!(try_offset(hours), Err(ShellError::TypeMismatch { .. })),
                "{hours}"
            );
        }
        let actual = into_datetime(Value::test_int(0), offset(-12), None);
        assert_eq!(actual, rfc3339("1969-12-31T12:00:00-12:00"));
    }

    /// A name that is no zone is an error; any case of a real one is that zone.
    #[test]
    fn reads_zone_names() {
        assert!(matches!(
            try_timezone("Nowhere/Town"),
            Err(ShellError::TypeMismatch { .. })
        ));
        // Date equality compares instants, so the offset is checked on its own.
        for (name, hours) in [("u", 0), ("UTC", 0), ("aSiA/tOkYo", 9)] {
            let actual = into_datetime(
                Value::test_int(1_614_434_140_000_000_000),
                timezone(name),
                None,
            );
            assert_eq!(
                actual.as_date().map(DateTime::offset),
                Ok(Offset::constant(hours)),
                "{name}"
            );
        }
    }

    /// `--offset` wins over `--timezone`, which is then not looked up.
    #[test]
    fn offset_takes_precedence_over_timezone() {
        let zone = zone_flag(
            Some(3.into_spanned(Span::test_data())),
            Some("Nowhere/Town".to_string().into_spanned(Span::test_data())),
        );
        assert!(matches!(zone, Ok(Some(_))));
    }

    /// A wall-clock time that is in range at UTC but not in the zone names the input in its
    /// error, a string or an int alike.
    #[test]
    fn wall_clock_out_of_range_in_the_zone() {
        for (input, format, text) in [
            (
                Value::test_string("9999-12-30 20:00"),
                "%F %R",
                "9999-12-30 20:00",
            ),
            (
                Value::test_int(999_912_302_000),
                "%Y%m%d%H%M",
                "999912302000",
            ),
        ] {
            let actual = into_datetime(input, offset(-12), Some(format));
            assert!(
                matches!(
                    actual.unwrap_error(),
                    Err(ShellError::DatetimeParseError { ref msg, .. }) if *msg == text
                ),
                "{text}"
            );
        }
    }

    /// Every path gives the date the input's span, with or without a zone flag, not the flag's
    /// or the command's.
    #[test]
    fn result_has_the_input_span() {
        let input_span = Span::new(1, 2);
        let head = Span::new(3, 4);
        let format = |format: &str| Some(format.to_string().into_spanned(Span::new(5, 6)));
        let zone_span = Span::new(7, 8);
        for (input, format) in [
            (Value::string("2024-01-01 10:00", input_span), None),
            (Value::string("1614434140", input_span), None),
            (Value::int(1614434140, input_span), None),
            (Value::string("2024-01-01", input_span), format("%F")),
            (
                Value::string("2024-01-01 +0000", input_span),
                format("%F %z"),
            ),
            (Value::int(1614434140, input_span), format("%s")),
        ] {
            for zone in [
                None,
                Some(TimeZone::UTC.into_spanned(zone_span)),
                Some(TimeZone::fixed(Offset::constant(8)).into_spanned(zone_span)),
            ] {
                let args = Arguments {
                    zone,
                    format: format.clone(),
                    cell_paths: None,
                };
                let actual = action(&input, &args, head);
                assert!(
                    matches!(actual, Value::Date { .. }),
                    "{input:?}: {actual:?}"
                );
                assert_eq!(actual.span(), input_span, "{input:?}");
            }
        }
    }
}
