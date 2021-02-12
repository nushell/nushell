use crate::prelude::*;
use chrono::naive::NaiveDate;
use chrono::{Duration, Local};
use nu_engine::WholeStreamCommand;
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        seq_dates(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "print the next 10 days in YYYY-MM-DD format with newline separator",
                example: "seq date --days 10",
                result: None,
            },
            Example {
                description: "print the previous 10 days in YYYY-MM-DD format with newline separator",
                example: "seq date --days 10 -r",
                result: None,
            },
            Example {
                description: "print the previous 10 days starting today in MM/DD/YYYY format with newline separator",
                example: "seq date --days 10 -o '%m/%d/%Y' -r",
                result: None,
            },
            Example {
                description: "print the first 10 days in January, 2020",
                example: "seq date -b '2020-01-01' -e '2020-01-10'",
                result: Some(vec![
                    UntaggedValue::string("2020-01-01").into(),
                    UntaggedValue::string("2020-01-02").into(),
                    UntaggedValue::string("2020-01-03").into(),
                    UntaggedValue::string("2020-01-04").into(),
                    UntaggedValue::string("2020-01-05").into(),
                    UntaggedValue::string("2020-01-06").into(),
                    UntaggedValue::string("2020-01-07").into(),
                    UntaggedValue::string("2020-01-08").into(),
                    UntaggedValue::string("2020-01-09").into(),
                    UntaggedValue::string("2020-01-10").into(),
                ]),
            },
            Example {
                description: "print every fifth day between January 1st 2020 and January 31st 2020",
                example: "seq date -b '2020-01-01' -e '2020-01-31' -n 5",
                result: Some(vec![
                    UntaggedValue::string("2020-01-01").into(),
                    UntaggedValue::string("2020-01-06").into(),
                    UntaggedValue::string("2020-01-11").into(),
                    UntaggedValue::string("2020-01-16").into(),
                    UntaggedValue::string("2020-01-21").into(),
                    UntaggedValue::string("2020-01-26").into(),
                    UntaggedValue::string("2020-01-31").into(),
                ]),
            },
            Example {
                description: "starting on May 5th, 2020, print the next 10 days in your locale's date format, colon separated",
                example: "seq date -o %x -s ':' -d 10 -b '2020-05-01'",
                result: None,
            },
        ]
    }
}

async fn seq_dates(args: CommandArgs) -> Result<OutputStream, ShellError> {
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
    ) = args.process().await?;

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
        _ => (1_i64).to_value_create_tag(),
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

#[allow(clippy::too_many_arguments)]
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
        step_size *= -1;
        days_to_output *= -1;
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

    let is_out_of_range =
        |next| (step_size > 0 && next > end_date) || (step_size < 0 && next < end_date);

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

        ret_str.push_str(&separator);
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

        test_examples(SeqDates {})
    }
}
