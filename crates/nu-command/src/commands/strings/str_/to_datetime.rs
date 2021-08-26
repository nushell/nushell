use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, ShellTypeName, Signature, SyntaxShape, UntaggedValue,
    Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

use chrono::{DateTime, FixedOffset, Local, LocalResult, Offset, TimeZone, Utc};

struct Arguments {
    timezone: Option<Tagged<String>>,
    offset: Option<Tagged<i16>>,
    format: Option<Tagged<String>>,
    column_paths: Vec<ColumnPath>,
}

// In case it may be confused with chrono::TimeZone
#[derive(Clone)]
enum Zone {
    Utc,
    Local,
    East(u8),
    West(u8),
    Error, // we want the nullshell to cast it instead of rust
}

impl Zone {
    fn new(i: i16) -> Self {
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

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str to-datetime"
    }

    fn signature(&self) -> Signature {
        Signature::build("str to-datetime")
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
                SyntaxShape::Any,
                "optionally convert text into datetime by column paths",
            )
    }

    fn usage(&self) -> &str {
        "converts text into datetime"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to datetime",
                example: "echo '16.11.1984 8:00 am +0000' | str to-datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime",
                example: "echo '2020-08-04T16:39:18+00:00' | str to-datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime using a custom format",
                example: "echo '20200904_163918+0000' | str to-datetime -f '%Y%m%d_%H%M%S%z'",
                result: None,
            },
            Example {
                description: "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone",
                example: "echo '1614434140' | str to-datetime -z 'UTC'",
                result: None,
            },
            Example {
                description:
                    "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)",
                example: "echo '1614434140' | str to-datetime -o '+9'",
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
struct DatetimeFormat(String);

fn operate(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let options = Arguments {
        timezone: args.get_flag("timezone")?,
        offset: args.get_flag("offset")?,
        format: args.get_flag("format")?,
        column_paths: args.rest(0)?,
    };
    let input = args.input;

    // if zone-offset is specified, then zone will be neglected
    let zone_options = if let Some(Tagged {
        item: zone_offset,
        tag,
    }) = &options.offset
    {
        Some(Tagged {
            item: Zone::new(*zone_offset),
            tag: tag.into(),
        })
    } else if let Some(Tagged { item: zone, tag }) = &options.timezone {
        Some(Tagged {
            item: Zone::from_string(zone.clone()),
            tag: tag.into(),
        })
    } else {
        None
    };

    let format_options = if let Some(Tagged { item: fmt, .. }) = &options.format {
        Some(DatetimeFormat(fmt.to_string()))
    } else {
        None
    };

    Ok(input
        .map(move |v| {
            if options.column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &zone_options, &format_options, v.tag())?)
            } else {
                let mut ret = v;

                for path in &options.column_paths {
                    let zone_options = zone_options.clone();
                    let format_options = format_options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &zone_options, &format_options, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .into_action_stream())
}

fn action(
    input: &Value,
    timezone: &Option<Tagged<Zone>>,
    dateformat: &Option<DatetimeFormat>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let ts = s.parse::<i64>();
            // if timezone if specified, first check if the input is a timestamp.
            if let Some(tz) = timezone {
                const TIMESTAMP_BOUND: i64 = 8.2e+12 as i64;
                // Since the timestamp method of chrono itself don't throw an error (it just panicked)
                // We have to manually guard it.
                if let Ok(t) = ts {
                    if t.abs() > TIMESTAMP_BOUND {
                        return Err(ShellError::labeled_error(
                            "could not parse input as a valid timestamp",
                            "given timestamp is out of range, it should between -8e+12 and 8e+12",
                            tag.into().span,
                        ));
                    }
                    const HOUR: i32 = 3600;
                    let stampout = match tz.item {
                        Zone::Utc => UntaggedValue::date(Utc.timestamp(t, 0)),
                        Zone::Local => UntaggedValue::date(Local.timestamp(t, 0)),
                        Zone::East(i) => {
                            let eastoffset = FixedOffset::east((i as i32) * HOUR);
                            UntaggedValue::date(eastoffset.timestamp(t, 0))
                        }
                        Zone::West(i) => {
                            let westoffset = FixedOffset::west((i as i32) * HOUR);
                            UntaggedValue::date(westoffset.timestamp(t, 0))
                        }
                        Zone::Error => {
                            return Err(ShellError::labeled_error(
                                "could not continue to convert timestamp",
                                "given timezone or offset is invalid",
                                tz.tag().span,
                            ));
                        }
                    };
                    return Ok(stampout.into_value(tag));
                }
            };
            // if it's not, continue and negelect the timezone option.
            let out = match dateformat {
                Some(dt) => match DateTime::parse_from_str(s, &dt.0) {
                    Ok(d) => UntaggedValue::date(d),
                    Err(reason) => {
                        return Err(ShellError::labeled_error(
                            format!("could not parse as datetime using format '{}'", dt.0),
                            reason.to_string(),
                            tag.into().span,
                        ))
                    }
                },
                None => match dtparse::parse(s) {
                    Ok((native_dt, fixed_offset)) => {
                        let offset = match fixed_offset {
                            Some(fo) => fo,
                            None => FixedOffset::east(0).fix(),
                        };
                        match offset.from_local_datetime(&native_dt) {
                            LocalResult::Single(d) => UntaggedValue::date(d),
                            LocalResult::Ambiguous(d, _) => UntaggedValue::date(d),
                            LocalResult::None => {
                                return Err(ShellError::labeled_error(
                                    "could not convert to a timezone-aware datetime",
                                    "local time representation is invalid",
                                    tag.into().span,
                                ))
                            }
                        }
                    }
                    Err(reason) => {
                        return Err(ShellError::labeled_error(
                            "could not parse as datetime",
                            reason.to_string(),
                            tag.into().span,
                        ))
                    }
                },
            };

            Ok(out.into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{action, DatetimeFormat, SubCommand, Zone};
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::{Tag, Tagged};
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn takes_a_date_format() {
        let date_str = string("16.11.1984 8:00 am +0000");

        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));

        let actual = action(&date_str, &None, &fmt_options, Tag::unknown()).unwrap();

        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = string("2020-08-04T16:39:18+00:00");
        let actual = action(&date_str, &None, &None, Tag::unknown()).unwrap();
        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn takes_timestamp_offset() {
        let date_str = string("1614434140");
        let timezone_option = Some(Tagged {
            item: Zone::East(8),
            tag: Tag::unknown(),
        });
        let actual = action(&date_str, &timezone_option, &None, Tag::unknown()).unwrap();
        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn takes_timestamp() {
        let date_str = string("1614434140");
        let timezone_option = Some(Tagged {
            item: Zone::Local,
            tag: Tag::unknown(),
        });
        let actual = action(&date_str, &timezone_option, &None, Tag::unknown()).unwrap();
        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn takes_invalid_timestamp() {
        let date_str = string("10440970000000");
        let timezone_option = Some(Tagged {
            item: Zone::Utc,
            tag: Tag::unknown(),
        });
        let actual = action(&date_str, &timezone_option, &None, Tag::unknown());

        assert!(actual.is_err());
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = string("16.11.1984 8:00 am Oops0000");

        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));

        let actual = action(&date_str, &None, &fmt_options, Tag::unknown());

        assert!(actual.is_err());
    }
}
