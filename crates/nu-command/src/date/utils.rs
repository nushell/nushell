use chrono::{DateTime, FixedOffset, Local, LocalResult, TimeZone};
use nu_protocol::{ShellError, Span, Value, record};

pub(crate) fn parse_date_from_string(
    input: &str,
    span: Span,
) -> Result<DateTime<FixedOffset>, Value> {
    match dtparse::parse(input) {
        Ok((native_dt, fixed_offset)) => {
            let offset = match fixed_offset {
                Some(offset) => offset,
                None => *Local
                    .from_local_datetime(&native_dt)
                    .single()
                    .unwrap_or_default()
                    .offset(),
            };
            match offset.from_local_datetime(&native_dt) {
                LocalResult::Single(d) => Ok(d),
                LocalResult::Ambiguous(d, _) => Ok(d),
                LocalResult::None => Err(Value::error(
                    ShellError::DatetimeParseError {
                        msg: input.into(),
                        span,
                    },
                    span,
                )),
            }
        }
        Err(_) => Err(Value::error(
            ShellError::DatetimeParseError {
                msg: input.into(),
                span,
            },
            span,
        )),
    }
}

/// Generates a table containing available datetime format specifiers
///
/// # Arguments
/// * `head` - use the call's head
/// * `show_parse_only_formats` - whether parse-only format specifiers (that can't be outputted) should be shown. Should only be used for `into datetime`, not `format date`
pub(crate) fn generate_strftime_list(head: Span, show_parse_only_formats: bool) -> Value {
    let now = Local::now();

    struct FormatSpecification<'a> {
        spec: &'a str,
        description: &'a str,
    }

    let specifications = [
        FormatSpecification {
            spec: "%Y",
            description: "The full proleptic Gregorian year, zero-padded to 4 digits.",
        },
        FormatSpecification {
            spec: "%C",
            description: "The proleptic Gregorian year divided by 100, zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%y",
            description: "The proleptic Gregorian year modulo 100, zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%m",
            description: "Month number (01--12), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%b",
            description: "Abbreviated month name. Always 3 letters.",
        },
        FormatSpecification {
            spec: "%B",
            description: "Full month name. Also accepts corresponding abbreviation in parsing.",
        },
        FormatSpecification {
            spec: "%h",
            description: "Same as %b.",
        },
        FormatSpecification {
            spec: "%d",
            description: "Day number (01--31), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%e",
            description: "Same as %d but space-padded. Same as %_d.",
        },
        FormatSpecification {
            spec: "%a",
            description: "Abbreviated weekday name. Always 3 letters.",
        },
        FormatSpecification {
            spec: "%A",
            description: "Full weekday name. Also accepts corresponding abbreviation in parsing.",
        },
        FormatSpecification {
            spec: "%w",
            description: "Sunday = 0, Monday = 1, ..., Saturday = 6.",
        },
        FormatSpecification {
            spec: "%u",
            description: "Monday = 1, Tuesday = 2, ..., Sunday = 7. (ISO 8601)",
        },
        FormatSpecification {
            spec: "%U",
            description: "Week number starting with Sunday (00--53), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%W",
            description: "Same as %U, but week 1 starts with the first Monday in that year instead.",
        },
        FormatSpecification {
            spec: "%G",
            description: "Same as %Y but uses the year number in ISO 8601 week date.",
        },
        FormatSpecification {
            spec: "%g",
            description: "Same as %y but uses the year number in ISO 8601 week date.",
        },
        FormatSpecification {
            spec: "%V",
            description: "Same as %U but uses the week number in ISO 8601 week date (01--53).",
        },
        FormatSpecification {
            spec: "%j",
            description: "Day of the year (001--366), zero-padded to 3 digits.",
        },
        FormatSpecification {
            spec: "%D",
            description: "Month-day-year format. Same as %m/%d/%y.",
        },
        FormatSpecification {
            spec: "%x",
            description: "Locale's date representation (e.g., 12/31/99).",
        },
        FormatSpecification {
            spec: "%F",
            description: "Year-month-day format (ISO 8601). Same as %Y-%m-%d.",
        },
        FormatSpecification {
            spec: "%v",
            description: "Day-month-year format. Same as %e-%b-%Y.",
        },
        FormatSpecification {
            spec: "%H",
            description: "Hour number (00--23), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%k",
            description: "Same as %H but space-padded. Same as %_H.",
        },
        FormatSpecification {
            spec: "%I",
            description: "Hour number in 12-hour clocks (01--12), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%l",
            description: "Same as %I but space-padded. Same as %_I.",
        },
        FormatSpecification {
            spec: "%P",
            description: "am or pm in 12-hour clocks.",
        },
        FormatSpecification {
            spec: "%p",
            description: "AM or PM in 12-hour clocks.",
        },
        FormatSpecification {
            spec: "%M",
            description: "Minute number (00--59), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%S",
            description: "Second number (00--60), zero-padded to 2 digits.",
        },
        FormatSpecification {
            spec: "%f",
            description: "The fractional seconds (in nanoseconds) since last whole second.",
        },
        FormatSpecification {
            spec: "%.f",
            description: "Similar to .%f but left-aligned. These all consume the leading dot.",
        },
        FormatSpecification {
            spec: "%.3f",
            description: "Similar to .%f but left-aligned but fixed to a length of 3.",
        },
        FormatSpecification {
            spec: "%.6f",
            description: "Similar to .%f but left-aligned but fixed to a length of 6.",
        },
        FormatSpecification {
            spec: "%.9f",
            description: "Similar to .%f but left-aligned but fixed to a length of 9.",
        },
        FormatSpecification {
            spec: "%3f",
            description: "Similar to %.3f but without the leading dot.",
        },
        FormatSpecification {
            spec: "%6f",
            description: "Similar to %.6f but without the leading dot.",
        },
        FormatSpecification {
            spec: "%9f",
            description: "Similar to %.9f but without the leading dot.",
        },
        FormatSpecification {
            spec: "%R",
            description: "Hour-minute format. Same as %H:%M.",
        },
        FormatSpecification {
            spec: "%T",
            description: "Hour-minute-second format. Same as %H:%M:%S.",
        },
        FormatSpecification {
            spec: "%X",
            description: "Locale's time representation (e.g., 23:13:48).",
        },
        FormatSpecification {
            spec: "%r",
            description: "Hour-minute-second format in 12-hour clocks. Same as %I:%M:%S %p.",
        },
        FormatSpecification {
            spec: "%Z",
            description: "Local time zone name. Skips all non-whitespace characters during parsing.",
        },
        FormatSpecification {
            spec: "%z",
            description: "Offset from the local time to UTC (with UTC being +0000).",
        },
        FormatSpecification {
            spec: "%:z",
            description: "Same as %z but with a colon.",
        },
        FormatSpecification {
            spec: "%c",
            description: "Locale's date and time (e.g., Thu Mar 3 23:05:25 2005).",
        },
        FormatSpecification {
            spec: "%+",
            description: "ISO 8601 / RFC 3339 date & time format.",
        },
        FormatSpecification {
            spec: "%s",
            description: "UNIX timestamp, the number of seconds since 1970-01-01",
        },
        FormatSpecification {
            spec: "%J",
            description: "Joined date format. Same as %Y%m%d.",
        },
        FormatSpecification {
            spec: "%Q",
            description: "Sequential time format. Same as %H%M%S.",
        },
        FormatSpecification {
            spec: "%t",
            description: "Literal tab (\\t).",
        },
        FormatSpecification {
            spec: "%n",
            description: "Literal newline (\\n).",
        },
        FormatSpecification {
            spec: "%%",
            description: "Literal percent sign.",
        },
    ];

    let mut records = specifications
        .iter()
        .map(|s| {
            // Handle custom format specifiers that aren't supported by chrono
            let example = match s.spec {
                "%J" => now.format("%Y%m%d").to_string(),
                "%Q" => now.format("%H%M%S").to_string(),
                _ => now.format(s.spec).to_string(),
            };

            Value::record(
                record! {
                    "Specification" => Value::string(s.spec, head),
                    "Example" => Value::string(example, head),
                    "Description" => Value::string(s.description, head),
                },
                head,
            )
        })
        .collect::<Vec<Value>>();

    if show_parse_only_formats {
        // now.format("%#z") will panic since it is parse-only
        // so here we emulate how it will look:
        let example = now
            .format("%:z") // e.g. +09:30
            .to_string()
            .get(0..3) // +09:30 -> +09
            .unwrap_or("")
            .to_string();

        let description = "Parsing only: Same as %z but allows minutes to be missing or present.";

        records.push(Value::record(
            record! {
                "Specification" => Value::string("%#z", head),
                "Example" => Value::string(example, head),
                "Description" => Value::string(description, head),
            },
            head,
        ));
    }

    Value::list(records, head)
}
