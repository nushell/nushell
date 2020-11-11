use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use chrono::naive::NaiveDate;
use chrono::{Duration, Local};
use nu_errors::ShellError;
use nu_protocol::{value::I64Ext, value::StrExt, value::StringExt, value::U64Ext};
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SeqDates;

#[derive(Deserialize)]
pub struct SeqDatesArgs {
    separator: Option<Tagged<String>>,
    output_format: Option<Tagged<String>>,
    input_format: Option<Tagged<String>>,
    begin_date: Option<Tagged<String>>,
    end_date: Option<Tagged<String>>,
    increment: Option<Tagged<i64>>,
    days: Option<Tagged<u64>>,
    reverse: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for SeqDates {
    fn name(&self) -> &str {
        "seq date"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq date")
            .named(
                "separator",
                SyntaxShape::String,
                "separator character (defaults to \\n)",
                Some('s'),
            )
            .named(
                "output_format",
                SyntaxShape::String,
                "prints dates in this format (defaults to %Y-%m-%d)",
                Some('o'),
            )
            .named(
                "input_format",
                SyntaxShape::String,
                "give argument dates in this format (defaults to %Y-%m-%d)",
                Some('i'),
            )
            .named(
                "begin_date",
                SyntaxShape::String,
                "beginning date range",
                Some('b'),
            )
            .named("end_date", SyntaxShape::String, "ending date", Some('e'))
            .named(
                "increment",
                SyntaxShape::Int,
                "increment dates by this number",
                Some('n'),
            )
            .named(
                "days",
                SyntaxShape::Int,
                "number of days to print",
                Some('d'),
            )
            .switch("reverse", "print dates in reverse", Some('r'))
    }

    fn usage(&self) -> &str {
        "print sequences of dates"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        seq_dates(args, registry).await
    }
    // after_help("If only LAST is given, first defaults to today.  If INCREMENT is omitted, it\n \
    // defaults to 1. If FIRST is later than LAST, the sequence will be printed\n \
    // backward.  This is different than the seq command.  INCREMENT can not be zero,\n \
    // that makes no sense.\n\n \
    // FORMAT arguments to input and output must be suitable for strftime and strptime.\n\
    // The default format for both input and output is YYYY-MM-DD.\n\n\
    // Examples:\n\
    // Print the next 10 days in YYYY-MM-DD format\n\
    // $ dseq 10\n\
    // Print the last 10 days starting today in MM/DD/YYYY format\n\
    // $ dseq -o %m/%d/%Y -10\n\
    // Print the days in January, 2015\n\
    // $ dseq 2015-01-01 2015-01-31\n\
    // Print every fifth day between January 7th 2015 and May 9th 2015\n\
    // $ dseq 2015-01-07 5 2015-05-09\n\
    // Print the next 10 days in your locale's date format, comma separated\n\
    // $ dseq -o %x -s : 10\n")
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence dates 1 to 10 with newline separator",
                example: "seq date 1/1/1 1/1/10",
                result: Some(vec![
                    UntaggedValue::string("1").into(),
                    UntaggedValue::string("2").into(),
                    UntaggedValue::string("3").into(),
                    UntaggedValue::string("4").into(),
                    UntaggedValue::string("5").into(),
                    UntaggedValue::string("6").into(),
                    UntaggedValue::string("7").into(),
                    UntaggedValue::string("8").into(),
                    UntaggedValue::string("9").into(),
                    UntaggedValue::string("10").into(),
                ]),
            },
            Example {
                description: "sequence dates 1 to 10 with pipe separator",
                example: "seq date -s '|' 1 10",
                result: Some(vec![Value::from("1|2|3|4|5|6|7|8|9|10")]),
            },
            Example {
                description: "sequence dates 1 to 10 with pipe separator padded with 0",
                example: "seq dates -s '|' -w 1 10",
                result: Some(vec![Value::from("01|02|03|04|05|06|07|08|09|10")]),
            },
            Example {
                description: "sequence dates1 to 10 with pipe separator padded by 2s",
                example: "seq date -s ' | ' -w 1 2 10",
                result: Some(vec![Value::from("01 | 03 | 05 | 07 | 09")]),
            },
        ]
    }
}

async fn seq_dates(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let _name = args.call_info.name_tag.clone();

    let (
        SeqDatesArgs {
            separator,
            output_format,
            input_format,
            begin_date,
            end_date,
            increment,
            days,
            reverse,
        },
        _,
    ) = args.process(&registry).await?;

    let sep: String = match separator {
        Some(s) => {
            if s.item == r"\t" {
                '\t'.to_string()
            } else if s.item == r"\n" {
                '\n'.to_string()
            } else if s.item == r"\r" {
                '\r'.to_string()
            } else {
                let vec_s: Vec<char> = s.chars().collect();
                if vec_s.is_empty() {
                    return Err(ShellError::labeled_error(
                        "Expected a single separator char from --separator",
                        "requires a single character string input",
                        &s.tag,
                    ));
                };
                vec_s.iter().collect()
            }
        }
        _ => '\n'.to_string(),
    };

    let outformat = match output_format {
        Some(s) => Some(s.item.to_string_value(s.tag)),
        _ => None,
    };

    let informat = match input_format {
        Some(s) => Some(s.item.to_string_value(s.tag)),
        _ => None,
    };

    let begin = match begin_date {
        Some(s) => Some(s.item),
        _ => None,
    };

    let end = match end_date {
        Some(s) => Some(s.item),
        _ => None,
    };

    let inc = match increment {
        Some(i) => {
            let clone = i.clone();
            i.to_value(clone.tag)
        }
        _ => (1 as i64).to_value_create_tag(),
    };

    let day_count: Option<Value> = match days {
        Some(i) => Some(i.item.to_value(i.tag)),
        _ => None,
    };

    let mut rev = false;
    if *reverse {
        rev = *reverse;
    }

    run_seq_dates(sep, outformat, informat, begin, end, inc, day_count, rev)
}

pub fn parse_date_string(s: &str, format: &str) -> Result<NaiveDate, &'static str> {
    let d = match NaiveDate::parse_from_str(s, format) {
        Ok(d) => d,
        Err(_) => return Err("Failed to parse date."),
    };
    Ok(d)
}

pub fn run_seq_dates(
    separator: String,
    output_format: Option<Value>,
    input_format: Option<Value>,
    beginning_date: Option<String>,
    ending_date: Option<String>,
    increment: Value,
    day_count: Option<Value>,
    reverse: bool,
) -> Result<OutputStream, ShellError> {
    let today = Local::today().naive_local();
    let mut step_size: i64 = increment
        .as_i64()
        .expect("unable to change increment to i64");

    if step_size == 0 {
        return Err(ShellError::labeled_error(
            "increment cannot be 0",
            "increment cannot be 0",
            increment.tag,
        ));
    }

    let in_format = match input_format {
        Some(i) => i.as_string().map_err(|e| {
            ShellError::labeled_error(
                e.to_string(),
                "error with input_format as_string",
                i.tag.span,
            )
        })?,
        None => "%Y-%m-%d".to_string(),
    };

    let out_format = match output_format {
        Some(o) => o.as_string().map_err(|e| {
            ShellError::labeled_error(
                e.to_string(),
                "error with output_format as_string",
                o.tag.span,
            )
        })?,
        None => "%Y-%m-%d".to_string(),
    };

    let start_date = match beginning_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::labeled_error(
                    e,
                    "Failed to parse date",
                    Tag::unknown(),
                ))
            }
        },
        _ => today,
    };

    let mut end_date = match ending_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::labeled_error(
                    e,
                    "Failed to parse date",
                    Tag::unknown(),
                ))
            }
        },
        _ => today,
    };

    let mut days_to_output = match day_count {
        Some(d) => d.as_i64()?,
        None => 0i64,
    };

    // Make the signs opposite if we're created dates in reverse direction
    if reverse {
        step_size = step_size * -1;
        days_to_output = days_to_output * -1;
    }

    if days_to_output != 0 {
        end_date = match start_date.checked_add_signed(Duration::days(days_to_output)) {
            Some(date) => date,
            None => {
                return Err(ShellError::labeled_error(
                    "integer value too large",
                    "integer value too large",
                    Tag::unknown(),
                ));
            }
        }
    }

    // conceptually counting down with a positive step or counting up with a negative step
    // makes no sense, attempt to do what one means by inverting the signs in those cases.
    if (start_date > end_date) && (step_size > 0) || (start_date < end_date) && step_size < 0 {
        step_size = -step_size;
    }

    let is_out_of_range = |next| {
        if (step_size > 0 && next > end_date) || (step_size < 0 && next < end_date) {
            true
        } else {
            false
        }
    };

    let mut next = start_date;
    if is_out_of_range(next) {
        return Err(ShellError::labeled_error(
            "date is out of range",
            "date is out of range",
            Tag::unknown(),
        ));
    }

    let mut ret_str = String::from("");
    loop {
        ret_str.push_str(&format!("{}", next.format(&out_format)));
        // TODO: check this value is good
        next += Duration::days(step_size);

        if is_out_of_range(next) {
            break;
        }

        ret_str.push_str(&format!("{}", separator));
    }

    let rows: Vec<Value> = ret_str
        .lines()
        .map(|v| v.to_str_value_create_tag())
        .collect();
    Ok(futures::stream::iter(rows.into_iter().map(ReturnSuccess::value)).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::SeqDates;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(SeqDates {})?)
    }
}
