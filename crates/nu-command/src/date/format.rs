use chrono::Local;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Signature, Span, Spanned, SyntaxShape, Value,
};

use super::utils::{parse_date_from_string, unsupported_input_error};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .switch(
                "list",
                "lists strftime cheatsheet",
                Some('l')
            )
            .optional(
                "format string",
                SyntaxShape::String,
                "the desired date format",
            )
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Format a given date using the given format string."
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
            return Ok(PipelineData::Value(generate_strfttime_list(head), None))
        }
        let formatter = call.opt::<String>(engine_state, stack, 0)?
            .unwrap_or("%c".to_string());
        input.map(
            move |value| format_helper(value, &formatter, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Format a given date using the given format string.",
                example: "date format '%Y-%m-%d'",
                result: None,
            },
            Example {
                description: "Format a given date using the given format string.",
                example: r#"date format "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Format a given date using the given format string.",
                example: r#""2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d""#,
                result: None,
            },
        ]
    }
}

fn format_helper(value: Value, formatter: &str, span: Span) -> Value {
    match value {
        Value::Date { val, span: _ } => Value::String {
            val: val.format(formatter).to_string(),
            span,
        },
        Value::String {
            val,
            span: val_span,
        } => {
            let dt = parse_date_from_string(val, val_span);
            match dt {
                Ok(x) => Value::String {
                    val: x.format(formatter).to_string(),
                    span,
                },
                Err(e) => e,
            }
        }
        Value::Nothing { span: _ } => {
            let dt = Local::now();
            Value::String {
                val: dt
                    .with_timezone(dt.offset())
                    .format(formatter)
                    .to_string(),
                span,
            }
        }
        _ => unsupported_input_error(span),
    }
}

pub(crate) fn generate_strfttime_list(head: Span) -> Value {
    let column_names = vec![
        "Specification".into(),
        "Example".into(),
        "Description".into(),
    ];
    let records = vec![
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%Y".into(),
                    span: head,
                },
                Value::String {
                    val: "2001".into(),
                    span: head,
                },
                Value::String {
                    val: "The full proleptic Gregorian year, zero-padded to 4 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%C".into(),
                    span: head,
                },
                Value::String {
                    val: "20".into(),
                    span: head,
                },
                Value::String {
                    val: "The proleptic Gregorian year divided by 100, zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%y".into(),
                    span: head,
                },
                Value::String {
                    val: "01".into(),
                    span: head,
                },
                Value::String {
                    val: "The proleptic Gregorian year modulo 100, zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%m".into(),
                    span: head,
                },
                Value::String {
                    val: "07".into(),
                    span: head,
                },
                Value::String {
                    val: "Month number (01--12), zero-padded to 2 digits.".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%b".into(),
                    span: head,
                },
                Value::String {
                    val: "Jul".into(),
                    span: head,
                },
                Value::String {
                    val: "Abbreviated month name. Always 3 letters".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%B".into(),
                    span: head,
                },
                Value::String {
                    val: "July".into(),
                    span: head,
                },
                Value::String {
                    val: "Full month name. Also accepts corresponding abbreviation in parsing"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%h".into(),
                    span: head,
                },
                Value::String {
                    val: "Jul".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %b".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%d".into(),
                    span: head,
                },
                Value::String {
                    val: "08".into(),
                    span: head,
                },
                Value::String {
                    val: "Day number (01--31), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%e".into(),
                    span: head,
                },
                Value::String {
                    val: "8".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %d but space-padded. Same to %_d".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%a".into(),
                    span: head,
                },
                Value::String {
                    val: "Sun".into(),
                    span: head,
                },
                Value::String {
                    val: "Abbreviated weekday name. Always 3 letters".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%A".into(),
                    span: head,
                },
                Value::String {
                    val: "Sunday".into(),
                    span: head,
                },
                Value::String {
                    val: "Full weekday name. Also accepts corresponding abbreviation in parsing"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%w".into(),
                    span: head,
                },
                Value::String {
                    val: "0".into(),
                    span: head,
                },
                Value::String {
                    val: "Sunday = 0, Monday = 1, ..., Saturday = 6".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%u".into(),
                    span: head,
                },
                Value::String {
                    val: "7".into(),
                    span: head,
                },
                Value::String {
                    val: "Monday = 1, Tuesday = 2, ..., Sunday = 7. (ISO 8601)
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%U".into(),
                    span: head,
                },
                Value::String {
                    val: "28".into(),
                    span: head,
                },
                Value::String {
                    val: "Week number starting with Sunday (00--53), zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%W".into(),
                    span: head,
                },
                Value::String {
                    val: "27".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %U, but week 1 starts with the first Monday in that year instead"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%G".into(),
                    span: head,
                },
                Value::String {
                    val: "2001".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %Y but uses the year number in ISO 8601 week date. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%g".into(),
                    span: head,
                },
                Value::String {
                    val: "01".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %y but uses the year number in ISO 8601 week date. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%V".into(),
                    span: head,
                },
                Value::String {
                    val: "27".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %U but uses the week number in ISO 8601 week date (01--53). 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%j".into(),
                    span: head,
                },
                Value::String {
                    val: "189".into(),
                    span: head,
                },
                Value::String {
                    val: "Day of the year (001--366), zero-padded to 3 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%D".into(),
                    span: head,
                },
                Value::String {
                    val: "07/08/01".into(),
                    span: head,
                },
                Value::String {
                    val: "Month-day-year format. Same to %m/%d/%y".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%x".into(),
                    span: head,
                },
                Value::String {
                    val: "07/08/01".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %D".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%F".into(),
                    span: head,
                },
                Value::String {
                    val: "2001-07-08".into(),
                    span: head,
                },
                Value::String {
                    val: "Year-month-day format (ISO 8601). Same to %Y-%m-%d".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%v".into(),
                    span: head,
                },
                Value::String {
                    val: "8-Jul-2001".into(),
                    span: head,
                },
                Value::String {
                    val: "Day-month-year format. Same to %e-%b-%Y".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%H".into(),
                    span: head,
                },
                Value::String {
                    val: "00".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour number (00--23), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%k".into(),
                    span: head,
                },
                Value::String {
                    val: "0".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %H but space-padded. Same to %_H".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%I".into(),
                    span: head,
                },
                Value::String {
                    val: "12".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour number in 12-hour clocks (01--12), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%l".into(),
                    span: head,
                },
                Value::String {
                    val: "12".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %I but space-padded. Same to %_I".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%P".into(),
                    span: head,
                },
                Value::String {
                    val: "am".into(),
                    span: head,
                },
                Value::String {
                    val: "am or pm in 12-hour clocks".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%p".into(),
                    span: head,
                },
                Value::String {
                    val: "AM".into(),
                    span: head,
                },
                Value::String {
                    val: "AM or PM in 12-hour clocks".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%M".into(),
                    span: head,
                },
                Value::String {
                    val: "34".into(),
                    span: head,
                },
                Value::String {
                    val: "Minute number (00--59), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%S".into(),
                    span: head,
                },
                Value::String {
                    val: "60".into(),
                    span: head,
                },
                Value::String {
                    val: "Second number (00--60), zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%f".into(),
                    span: head,
                },
                Value::String {
                    val: "026490000".into(),
                    span: head,
                },
                Value::String {
                    val: "The fractional seconds (in nanoseconds) since last whole second. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 3. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 6. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490000".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 9. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%R".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour-minute format. Same to %H:%M".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%T".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour-minute-second format. Same to %H:%M:%S".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%X".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %T".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%r".into(),
                    span: head,
                },
                Value::String {
                    val: "12:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "AM Hour-minute-second format in 12-hour clocks. Same to %I:%M:%S %p"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%Z".into(),
                    span: head,
                },
                Value::String {
                    val: "ACST".into(),
                    span: head,
                },
                Value::String {
                    val: "Formatting only: Local time zone name".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%z".into(),
                    span: head,
                },
                Value::String {
                    val: "+0930".into(),
                    span: head,
                },
                Value::String {
                    val: "Offset from the local time to UTC (with UTC being +0000)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%:".into(),
                    span: head,
                },
                Value::String {
                    val: "+09:30".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %z but with a colon".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%c".into(),
                    span: head,
                },
                Value::String {
                    val: "Sun".into(),
                    span: head,
                },
                Value::String {
                    val:
                        "Jul 8 00:34:60 2001 ctime date & time format. Same to %a %b %e %T %Y sans"
                            .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%s".into(),
                    span: head,
                },
                Value::String {
                    val: "994518299".into(),
                    span: head,
                },
                Value::String {
                    val: "UNIX timestamp, the number of seconds since 1970-01-01 00:00 UTC.".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%t".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "Literal tab (\\t)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%n".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "Literal newline (\\n)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names,
            vals: vec![
                Value::String {
                    val: "%%".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "percent sign".into(),
                    span: head,
                },
            ],
            span: head,
        },
    ];

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
