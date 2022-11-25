use chrono::{DateTime, Local, Locale, TimeZone};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};
use nu_utils::locale::get_system_locale_string;
use std::fmt::{Display, Write};

use super::utils::parse_date_from_string;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .input_output_types(vec![
                (Type::Date, Type::String),
                (Type::String, Type::String),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "lists strftime cheatsheet", Some('l'))
            .optional(
                "format string",
                SyntaxShape::String,
                "the desired date format",
            )
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Format a given date using a format string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fmt", "strftime"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        if call.has_flag("list") {
            return Ok(PipelineData::Value(
                generate_strftime_list(head, false),
                None,
            ));
        }

        let format = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        if input.is_nothing() {
            return Err(ShellError::UnsupportedInput(
                "Input was nothing. You must pipe an input to this command.".into(),
                head,
            ));
        }

        input.map(
            move |value| match &format {
                Some(format) => format_helper(value, format.item.as_str(), format.span, head),
                None => format_helper_rfc2822(value, head),
            },
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // TODO: This should work but does not; see https://github.com/nushell/nushell/issues/7032
            // Example {
            //     description: "Format a given date-time using the default format (RFC 2822).",
            //     example: r#"'2021-10-22 20:00:12 +01:00' | into datetime | date format"#,
            //     result: Some(Value::String {
            //         val: "Fri, 22 Oct 2021 20:00:12 +0100".to_string(),
            //         span: Span::test_data(),
            //     }),
            // },
            Example {
                description:
                    "Format a given date-time as a string using the default format (RFC 2822).",
                example: r#""2021-10-22 20:00:12 +01:00" | date format"#,
                result: Some(Value::String {
                    val: "Fri, 22 Oct 2021 20:00:12 +0100".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Format the current date-time using a given format string.",
                example: r#"date now | date format "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Format the current date using a given format string.",
                example: r#"date now | date format "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Format a given date using a given format string.",
                example: r#""2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d""#,
                result: Some(Value::String {
                    val: "2021-10-22".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn format_from<Tz: TimeZone>(date_time: DateTime<Tz>, formatter: &str, span: Span) -> Value
where
    Tz::Offset: Display,
{
    let mut formatter_buf = String::new();
    let locale: Locale = get_system_locale_string()
        .map(|l| l.replace('-', "_")) // `chrono::Locale` needs something like `xx_xx`, rather than `xx-xx`
        .unwrap_or_else(|| String::from("en_US"))
        .as_str()
        .try_into()
        .unwrap_or(Locale::en_US);
    let format = date_time.format_localized(formatter, locale);

    match formatter_buf.write_fmt(format_args!("{}", format)) {
        Ok(_) => Value::String {
            val: formatter_buf,
            span,
        },
        Err(_) => Value::Error {
            error: ShellError::UnsupportedInput("invalid format".to_string(), span),
        },
    }
}

fn format_helper(value: Value, formatter: &str, formatter_span: Span, head_span: Span) -> Value {
    match value {
        Value::Date { val, .. } => format_from(val, formatter, formatter_span),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, formatter_span);

            match dt {
                Ok(x) => format_from(x, formatter, formatter_span),
                Err(e) => e,
            }
        }
        _ => Value::Error {
            error: ShellError::DatetimeParseError(head_span),
        },
    }
}

fn format_helper_rfc2822(value: Value, span: Span) -> Value {
    match value {
        Value::Date { val, span: _ } => Value::String {
            val: val.to_rfc2822(),
            span,
        },
        Value::String {
            val,
            span: val_span,
        } => {
            let dt = parse_date_from_string(&val, val_span);
            match dt {
                Ok(x) => Value::String {
                    val: x.to_rfc2822(),
                    span,
                },
                Err(e) => e,
            }
        }
        _ => Value::Error {
            error: ShellError::DatetimeParseError(span),
        },
    }
}

/// Generates a table containing available datetime format specifiers
///
/// # Arguments
/// * `head` - use the call's head
/// * `show_parse_only_formats` - whether parse-only format specifiers (that can't be outputted) should be shown. Should only be used for `into datetime`, not `date format`
pub(crate) fn generate_strftime_list(head: Span, show_parse_only_formats: bool) -> Value {
    let column_names = vec![
        "Specification".into(),
        "Example".into(),
        "Description".into(),
    ];
    let now = Local::now();

    struct FormatSpecification<'a> {
        spec: &'a str,
        description: &'a str,
    }

    let specifications = vec![
        FormatSpecification {
            spec: "%Y",
            description: "The full proleptic Gregorian year, zero-padded to 4 digits.",
        },
        FormatSpecification {
            spec: "%C",
            description: "The proleptic Gregorian year divided by 100, zero-padded to 2 digits.",
        },
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
        .map(|s| Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: s.spec.to_string(),
                    span: head,
                },
                Value::String {
                    val: now.format(s.spec).to_string(),
                    span: head,
                },
                Value::String {
                    val: s.description.to_string(),
                    span: head,
                },
            ],
            span: head,
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

        records.push(Value::Record {
            cols: column_names,
            vals: vec![
                Value::String {
                    val: "%#z".to_string(),
                    span: head,
                },
                Value::String {
                    val: example,
                    span: head,
                },
                Value::String {
                    val: "Parsing only: Same as %z but allows minutes to be missing or present."
                        .to_string(),
                    span: head,
                },
            ],
            span: head,
        });
    }

    Value::List {
        vals: records,
        span: head,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
