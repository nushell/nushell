use chrono::{DateTime, FixedOffset, Local, LocalResult, Offset, TimeZone, Utc};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use crate::generate_strftime_list;

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
    Error, // we want the nullshell to cast it instead of rust
}

impl Zone {
    fn new(i: i64) -> Self {
        if i.abs() <= 12 {
            // guanranteed here
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
            .switch(
                "list",
                "lists strftime cheatsheet",
                Some('l'),
                )
            .named(
                "timezone",
                SyntaxShape::String,
                "Specify timezone if the input is timestamp, like 'UTC/u' or 'LOCAL/l'",
                Some('z'),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify timezone by offset if the input is timestamp, like '+8', '-4', prior than timezone",
                Some('o'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify date and time formatting",
                Some('f'),
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
        "converts text into datetime"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to datetime",
                example: "'16.11.1984 8:00 am +0000' | into datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime",
                example: "'2020-08-04T16:39:18+00:00' | into datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime using a custom format",
                example: "'20200904_163918+0000' | into datetime -f '%Y%m%d_%H%M%S%z'",
                result: None,
            },
            Example {
                description: "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone",
                example: "'1614434140' | into datetime -z 'UTC'",
                result: None,
            },
            Example {
                description:
                    "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)",
                example: "'1614434140' | into datetime -o +9",
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
                generate_strftime_list(head)
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
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(
    input: &Value,
    timezone: &Option<Spanned<Zone>>,
    dateformat: &Option<DatetimeFormat>,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, span, .. } => {
            let ts = s.parse::<i64>();
            // if timezone if specified, first check if the input is a timestamp.
            if let Some(tz) = timezone {
                const TIMESTAMP_BOUND: i64 = 8.2e+12 as i64;
                // Since the timestamp method of chrono itself don't throw an error (it just panicked)
                // We have to manually guard it.
                if let Ok(t) = ts {
                    if t.abs() > TIMESTAMP_BOUND {
                        return Value::Error{error: ShellError::UnsupportedInput(
                            "Given timestamp is out of range, it should between -8e+12 and 8e+12".to_string(),
                            head,
                        )};
                    }
                    const HOUR: i32 = 3600;
                    let stampout = match tz.item {
                        Zone::Utc => Value::Date {
                            val: Utc.timestamp(t, 0).into(),
                            span: head,
                        },
                        Zone::Local => Value::Date {
                            val: Local.timestamp(t, 0).into(),
                            span: head,
                        },
                        Zone::East(i) => {
                            let eastoffset = FixedOffset::east((i as i32) * HOUR);
                            Value::Date {
                                val: eastoffset.timestamp(t, 0),
                                span: head,
                            }
                        }
                        Zone::West(i) => {
                            let westoffset = FixedOffset::west((i as i32) * HOUR);
                            Value::Date {
                                val: westoffset.timestamp(t, 0),
                                span: head,
                            }
                        }
                        Zone::Error => Value::Error {
                            error: ShellError::UnsupportedInput(
                                "Cannot convert given timezone or offset to timestamp".to_string(),
                                tz.span,
                            ),
                        },
                    };
                    return stampout;
                }
            };
            // if it's not, continue and negelect the timezone option.
            let out = match dateformat {
                Some(dt) => match DateTime::parse_from_str(s, &dt.0) {
                    Ok(d) => Value::Date { val: d, span: head },
                    Err(reason) => {
                        return Value::Error {
                            error: ShellError::CantConvert(
                                format!("could not parse as datetime using format '{}'", dt.0),
                                reason.to_string(),
                                head,
                            ),
                        }
                    }
                },
                None => match dtparse::parse(s) {
                    Ok((native_dt, fixed_offset)) => {
                        let offset = match fixed_offset {
                            Some(fo) => fo,
                            None => FixedOffset::east(0).fix(),
                        };
                        match offset.from_local_datetime(&native_dt) {
                            LocalResult::Single(d) => Value::Date { val: d, span: head },
                            LocalResult::Ambiguous(d, _) => Value::Date { val: d, span: head },
                            LocalResult::None => {
                                return Value::Error {
                                    error: ShellError::CantConvert(
                                        "could not convert to a timezone-aware datetime"
                                            .to_string(),
                                        "local time representation is invalid".to_string(),
                                        head,
                                    ),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        return Value::Error {
                            error: ShellError::UnsupportedInput(
                                "Cannot convert input string as datetime. Might be missing timezone or offset".to_string(),
                                *span,
                            ),
                        }
                    }
                },
            };

            out
        }
        other => {
            let got = format!("Expected string, got {} instead", other.get_type());
            Value::Error {
                error: ShellError::UnsupportedInput(got, head),
            }
        }
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
        let date_str = Value::test_string("16.11.1984 8:00 am +0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("16.11.1984 8:00 am +0000", "%d.%m.%Y %H:%M %P %z")
                .unwrap(),
            span: Span::test_data(),
        };
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = Value::test_string("2020-08-04T16:39:18+00:00");
        let actual = action(&date_str, &None, &None, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("2020-08-04T16:39:18+00:00", "%Y-%m-%dT%H:%M:%S%z")
                .unwrap(),
            span: Span::test_data(),
        };
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset() {
        let date_str = Value::test_string("1614434140");
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z")
                .unwrap(),
            span: Span::test_data(),
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp() {
        let date_str = Value::test_string("1614434140");
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date {
            val: Local.timestamp(1614434140, 0).into(),
            span: Span::test_data(),
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_invalid_timestamp() {
        let date_str = Value::test_string("10440970000000");
        let timezone_option = Some(Spanned {
            item: Zone::Utc,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = Value::test_string("16.11.1984 8:00 am Oops0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }
}
