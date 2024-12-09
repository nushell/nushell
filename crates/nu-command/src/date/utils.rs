use chrono::{DateTime, FixedOffset, Local};
use dateparser::DateTimeUtc;
use nu_protocol::{record, ShellError, Span, Value};

pub(crate) fn parse_date_from_string(
    input: &str,
    span: Span,
) -> Result<DateTime<FixedOffset>, Value> {
    match input.parse::<DateTimeUtc>() {
        Ok(dt) => Ok(dt.0.with_timezone(&Local).into()),
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
            description:
                "Same as %U, but week 1 starts with the first Monday in that year instead.",
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
            description:
                "Local time zone name. Skips all non-whitespace characters during parsing.",
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
            Value::record(
                record! {
                    "Specification" => Value::string(s.spec, head),
                    "Example" => Value::string(now.format(s.spec).to_string(), head),
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

mod tests {

    #[test]
    fn test_dateparser_crate() {
        use dateparser::DateTimeUtc;

        // tests borrowed from https://github.com/waltzofpearls/dateparser
        let accepted = vec![
            // unix timestamp
            "1511648546",
            "1620021848429",
            "1620024872717915000",
            // rfc3339
            "2021-05-01T01:17:02.604456Z",
            "2017-11-25T22:34:50Z",
            // rfc2822
            "Wed, 02 Jun 2021 06:31:39 GMT",
            // postgres timestamp yyyy-mm-dd hh:mm:ss z
            "2019-11-29 08:08-08",
            "2019-11-29 08:08:05-08",
            "2021-05-02 23:31:36.0741-07",
            "2021-05-02 23:31:39.12689-07",
            "2019-11-29 08:15:47.624504-08",
            "2017-07-19 03:21:51+00:00",
            // yyyy-mm-dd hh:mm:ss
            "2014-04-26 05:24:37 PM",
            "2021-04-30 21:14",
            "2021-04-30 21:14:10",
            "2021-04-30 21:14:10.052282",
            "2014-04-26 17:24:37.123",
            "2014-04-26 17:24:37.3186369",
            "2012-08-03 18:31:59.257000000",
            // yyyy-mm-dd hh:mm:ss z
            "2017-11-25 13:31:15 PST",
            "2017-11-25 13:31 PST",
            "2014-12-16 06:20:00 UTC",
            "2014-12-16 06:20:00 GMT",
            "2014-04-26 13:13:43 +0800",
            "2014-04-26 13:13:44 +09:00",
            "2012-08-03 18:31:59.257000000 +0000",
            "2015-09-30 18:48:56.35272715 UTC",
            // yyyy-mm-dd
            "2021-02-21",
            // yyyy-mm-dd z
            "2021-02-21 PST",
            "2021-02-21 UTC",
            "2020-07-20+08:00",
            // hh:mm:ss
            "01:06:06",
            "4:00pm",
            "6:00 AM",
            // hh:mm:ss z
            "01:06:06 PST",
            "4:00pm PST",
            "6:00 AM PST",
            "6:00pm UTC",
            // Mon dd hh:mm:ss
            "May 6 at 9:24 PM",
            "May 27 02:45:27",
            // Mon dd, yyyy, hh:mm:ss
            "May 8, 2009 5:57:51 PM",
            "September 17, 2012 10:09am",
            "September 17, 2012, 10:10:09",
            // Mon dd, yyyy hh:mm:ss z
            "May 02, 2021 15:51:31 UTC",
            "May 02, 2021 15:51 UTC",
            "May 26, 2021, 12:49 AM PDT",
            "September 17, 2012 at 10:09am PST",
            // yyyy-mon-dd
            "2021-Feb-21",
            // Mon dd, yyyy
            "May 25, 2021",
            "oct 7, 1970",
            "oct 7, 70",
            "oct. 7, 1970",
            "oct. 7, 70",
            "October 7, 1970",
            // dd Mon yyyy hh:mm:ss
            "12 Feb 2006, 19:17",
            "12 Feb 2006 19:17",
            "14 May 2019 19:11:40.164",
            // dd Mon yyyy
            "7 oct 70",
            "7 oct 1970",
            "03 February 2013",
            "1 July 2013",
            // mm/dd/yyyy hh:mm:ss
            "4/8/2014 22:05",
            "04/08/2014 22:05",
            "4/8/14 22:05",
            "04/2/2014 03:00:51",
            "8/8/1965 12:00:00 AM",
            "8/8/1965 01:00:01 PM",
            "8/8/1965 01:00 PM",
            "8/8/1965 1:00 PM",
            "8/8/1965 12:00 AM",
            "4/02/2014 03:00:51",
            "03/19/2012 10:11:59",
            "03/19/2012 10:11:59.3186369",
            // mm/dd/yyyy
            "3/31/2014",
            "03/31/2014",
            "08/21/71",
            "8/1/71",
            // yyyy/mm/dd hh:mm:ss
            "2014/4/8 22:05",
            "2014/04/08 22:05",
            "2014/04/2 03:00:51",
            "2014/4/02 03:00:51",
            "2012/03/19 10:11:59",
            "2012/03/19 10:11:59.3186369",
            // yyyy/mm/dd
            "2014/3/31",
            "2014/03/31",
            // mm.dd.yyyy
            "3.31.2014",
            "03.31.2014",
            "08.21.71",
            // yyyy.mm.dd
            "2014.03.30",
            "2014.03",
            // yymmdd hh:mm:ss mysql log
            "171113 14:14:20",
            // chinese yyyy mm dd hh mm ss
            "2014年04月08日11时25分18秒",
            // chinese yyyy mm dd
            "2014年04月08日",
        ];

        for date_str in accepted {
            let result = date_str.parse::<DateTimeUtc>();
            // If you want to see the parsed date, uncomment the following lines
            // match dateparser::parse_with_timezone(date_str, &chrono::offset::Local) {
            //     Ok(dt) => {
            //         println!("\"{}\",\"{:?}\"", date_str, dt);
            //     }
            //     Err(e) => {
            //         println!("\n{}: {:?}\n", date_str, e);
            //     }
            // }
            assert!(result.is_ok())
        }
        assert!(true)
    }
}
