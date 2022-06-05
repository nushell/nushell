use chrono::{DateTime, FixedOffset, Local, TimeZone, Utc};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use crate::generate_strftime_list;
use crate::parse_date_from_string;

struct Arguments {
    timezone: Option<Spanned<String>>,
    offset: Option<Spanned<i64>>,
    format: Option<String>,
    column_paths: Vec<CellPath>,
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
        match s.to_lowercase().as_str() {
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
                "Specify an expected format for parsing strings to datetimes. Use --list to see all possible options",
                Some('f'),
            )
            .switch(
                "list",
                "Show all possible variables for use with the --format flag",
                Some('l'),
                )
            .rest(
            "rest",
                SyntaxShape::CellPath,
                "optionally convert text into datetime by column paths",
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
        operate(engine_state, stack, call, input)
    }

    fn usage(&self) -> &str {
        "Convert text into a datetime"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "date", "time", "timezone", "UTC"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to datetime",
                example: "'27.02.2021 1:55 pm +0000' | into datetime",
                result: Some(Value::Date (
                    Utc.timestamp(1614434100, 0).into(),
                )),
            },
            Example {
                description: "Convert to datetime",
                example: "'2021-02-27T13:55:40+00:00' | into datetime",
                result: Some(Value::Date(
                     Utc.timestamp(1614434140, 0).into(),
                )),
            },
            Example {
                description: "Convert to datetime using a custom format",
                example: "'20210227_135540+0000' | into datetime -f '%Y%m%d_%H%M%S%z'",
                result: Some(Value::Date (
                    Utc.timestamp(1614434140, 0).into(),
                )),
            },
            Example {
                description: "Convert timestamp (no larger than 8e+12) to a UTC datetime",
                example: "1614434140 | into datetime",
                result: Some(Value::Date (
                    Utc.timestamp(1614434140, 0).into(),
                )),
            },
            Example {
                description:
                    "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)",
                example: "1614434140 | into datetime -o +9",
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
struct DatetimeFormat(String);

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    let options = Arguments {
        timezone: call.get_flag(engine_state, stack, "timezone")?,
        offset: call.get_flag(engine_state, stack, "offset")?,
        format: call.get_flag(engine_state, stack, "format")?,
        column_paths: call.rest(engine_state, stack, 0)?,
    };

    // if zone-offset is specified, then zone will be neglected
    let zone_options = match &options.offset {
        Some(zone_offset) => Some(Spanned {
            item: Zone::new(zone_offset.item),
            span: zone_offset.span,
        }),
        None => options.timezone.as_ref().map(|zone| Spanned {
            item: Zone::from_string(zone.item.clone()),
            span: zone.span,
        }),
    };

    let list_flag = call.has_flag("list");

    let format_options = options
        .format
        .as_ref()
        .map(|fmt| DatetimeFormat(fmt.to_string()));

    input.map(
        move |v| {
            if options.column_paths.is_empty() && !list_flag {
                action(&v, &zone_options, &format_options, head)
            } else if list_flag {
                generate_strftime_list(head, true)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let zone_options = zone_options.clone();
                    let format_options = format_options.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &zone_options, &format_options, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error(error);
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
        head,
    )
}

fn action(
    input: &Value,
    timezone: &Option<Spanned<Zone>>,
    dateformat: &Option<DatetimeFormat>,
    head: Span,
) -> Value {
    // Check to see if input looks like a Unix timestamp (i.e. can it be parsed to an int?)
    let timestamp = match input {
        Value::Int(val) => Ok(*val),
        Value::String(val) => val.parse::<i64>(),
        other => {
            return Value::Error(ShellError::UnsupportedInput(
                format!("Expected string or int, got {} instead", other.get_type()),
                head,
            ));
        }
    };

    if let Ok(ts) = timestamp {
        const TIMESTAMP_BOUND: i64 = 8.2e+12 as i64;
        const HOUR: i32 = 3600;

        if ts.abs() > TIMESTAMP_BOUND {
            return Value::Error(ShellError::UnsupportedInput(
                "Given timestamp is out of range, it should between -8e+12 and 8e+12".to_string(),
                head,
            ));
        }

        return match timezone {
            // default to UTC
            None => Value::Date(Utc.timestamp(ts, 0).into()),
            Some(Spanned { item, span }) => match item {
                Zone::Utc => Value::Date(Utc.timestamp(ts, 0).into()),
                Zone::Local => Value::Date(Local.timestamp(ts, 0).into()),
                Zone::East(i) => {
                    let eastoffset = FixedOffset::east((*i as i32) * HOUR);
                    Value::Date(eastoffset.timestamp(ts, 0))
                }
                Zone::West(i) => {
                    let westoffset = FixedOffset::west((*i as i32) * HOUR);
                    Value::Date(westoffset.timestamp(ts, 0))
                }
                Zone::Error => Value::Error(ShellError::UnsupportedInput(
                    "Cannot convert given timezone or offset to timestamp".to_string(),
                    *span,
                )),
            },
        };
    }

    // If input is not a timestamp, try parsing it as a string
    match input {
        Value::String(val) => {
            match dateformat {
                Some(dt) => match DateTime::parse_from_str(val, &dt.0) {
                    Ok(d) => Value::Date( d ),
                    Err(reason) => {
                        return Value::Error (
                             ShellError::CantConvert(
                                format!("could not parse as datetime using format '{}'", dt.0),
                                reason.to_string(),
                                head,
                                Some("you can use `into datetime` without a format string to enable flexible parsing".to_string())
                            ),
                        )
                    }
                },
                // Tries to automatically parse the date
                // (i.e. without a format string)
                // and assumes the system's local timezone if none is specified
                None => match parse_date_from_string(val, head) {
                    Ok(date) => Value::Date(
                        date,
                    ),
                    Err(err) => err,
                },
            }
        }
        other => Value::Error(ShellError::UnsupportedInput(
            format!("Expected string, got {} instead", other.get_type()),
            head,
        )),
    }
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
        let date_str = Value::String("16.11.1984 8:00 am +0000".into());
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());
        let expected = Value::Date(
            DateTime::parse_from_str("16.11.1984 8:00 am +0000", "%d.%m.%Y %H:%M %P %z").unwrap(),
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = Value::String("2020-08-04T16:39:18+00:00".into());
        let actual = action(&date_str, &None, &None, Span::test_data());
        let expected = Value::Date(
            DateTime::parse_from_str("2020-08-04T16:39:18+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(),
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset() {
        let date_str = Value::String("1614434140".into());
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset_as_int() {
        let date_int = Value::Int(1614434141);
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let actual = action(&date_int, &timezone_option, &None, Span::test_data());
        let expected = Value::Date(
            DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z").unwrap(),
        );

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp() {
        let date_str = Value::String("1614434140".into());
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date(Local.timestamp(1614434140, 0).into());

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_without_timezone() {
        let date_str = Value::String("1614434140".into());
        let timezone_option = None;
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());

        let expected = Value::Date(Utc.timestamp(1614434140, 0).into());

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_invalid_timestamp() {
        let date_str = Value::String("10440970000000".into());
        let timezone_option = Some(Spanned {
            item: Zone::Utc,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = Value::String("16.11.1984 8:00 am Oops0000".into());
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }
}
