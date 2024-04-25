use chrono::{Duration, Local, NaiveDate};
use nu_engine::command_prelude::*;

use std::fmt::Write;

#[derive(Clone)]
pub struct SeqDate;

impl Command for SeqDate {
    fn name(&self) -> &str {
        "seq date"
    }

    fn usage(&self) -> &str {
        "Print sequences of dates."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("seq date")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .named(
                "output-format",
                SyntaxShape::String,
                "prints dates in this format (defaults to %Y-%m-%d)",
                Some('o'),
            )
            .named(
                "input-format",
                SyntaxShape::String,
                "give argument dates in this format (defaults to %Y-%m-%d)",
                Some('i'),
            )
            .named(
                "begin-date",
                SyntaxShape::String,
                "beginning date range",
                Some('b'),
            )
            .named("end-date", SyntaxShape::String, "ending date", Some('e'))
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
            .category(Category::Generators)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return a list of the next 10 days in the YYYY-MM-DD format",
                example: "seq date --days 10",
                result: None,
            },
            Example {
                description: "Return the previous 10 days in the YYYY-MM-DD format",
                example: "seq date --days 10 --reverse",
                result: None,
            },
            Example {
                description:
                    "Return the previous 10 days, starting today, in the MM/DD/YYYY format",
                example: "seq date --days 10 -o '%m/%d/%Y' --reverse",
                result: None,
            },
            Example {
                description: "Return the first 10 days in January, 2020",
                example: "seq date --begin-date '2020-01-01' --end-date '2020-01-10'",
                result: Some(Value::list(
                    vec![
                        Value::test_string("2020-01-01"),
                        Value::test_string("2020-01-02"),
                        Value::test_string("2020-01-03"),
                        Value::test_string("2020-01-04"),
                        Value::test_string("2020-01-05"),
                        Value::test_string("2020-01-06"),
                        Value::test_string("2020-01-07"),
                        Value::test_string("2020-01-08"),
                        Value::test_string("2020-01-09"),
                        Value::test_string("2020-01-10"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "print every fifth day between January 1st 2020 and January 31st 2020",
                example: "seq date --begin-date '2020-01-01' --end-date '2020-01-31' --increment 5",
                result: Some(Value::list(
                    vec![
                        Value::test_string("2020-01-01"),
                        Value::test_string("2020-01-06"),
                        Value::test_string("2020-01-11"),
                        Value::test_string("2020-01-16"),
                        Value::test_string("2020-01-21"),
                        Value::test_string("2020-01-26"),
                        Value::test_string("2020-01-31"),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let output_format: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "output-format")?;
        let input_format: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "input-format")?;
        let begin_date: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "begin-date")?;
        let end_date: Option<Spanned<String>> = call.get_flag(engine_state, stack, "end-date")?;
        let increment: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "increment")?;
        let days: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "days")?;
        let reverse = call.has_flag(engine_state, stack, "reverse")?;

        let outformat = match output_format {
            Some(s) => Some(Value::string(s.item, s.span)),
            _ => None,
        };

        let informat = match input_format {
            Some(s) => Some(Value::string(s.item, s.span)),
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
            Some(i) => Value::int(i.item, i.span),
            _ => Value::int(1_i64, call.head),
        };

        let day_count = days.map(|i| Value::int(i.item, i.span));

        let mut rev = false;
        if reverse {
            rev = reverse;
        }

        Ok(run_seq_dates(
            outformat, informat, begin, end, inc, day_count, rev, call.head,
        )?
        .into_pipeline_data())
    }
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
    output_format: Option<Value>,
    input_format: Option<Value>,
    beginning_date: Option<String>,
    ending_date: Option<String>,
    increment: Value,
    day_count: Option<Value>,
    reverse: bool,
    call_span: Span,
) -> Result<Value, ShellError> {
    let today = Local::now().date_naive();
    // if cannot convert , it will return error
    let mut step_size: i64 = increment.as_i64()?;

    if step_size == 0 {
        return Err(ShellError::GenericError {
            error: "increment cannot be 0".into(),
            msg: "increment cannot be 0".into(),
            span: Some(increment.span()),
            help: None,
            inner: vec![],
        });
    }

    let in_format = match input_format {
        Some(i) => match i.coerce_into_string() {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: "".into(),
                    span: None,
                    help: Some("error with input_format as_string".into()),
                    inner: vec![],
                });
            }
        },
        _ => "%Y-%m-%d".to_string(),
    };

    let out_format = match output_format {
        Some(o) => match o.coerce_into_string() {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: "".into(),
                    span: None,
                    help: Some("error with output_format as_string".into()),
                    inner: vec![],
                });
            }
        },
        _ => "%Y-%m-%d".to_string(),
    };

    let start_date = match beginning_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: "Failed to parse date".into(),
                    span: Some(call_span),
                    help: None,
                    inner: vec![],
                })
            }
        },
        _ => today,
    };

    let mut end_date = match ending_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: "Failed to parse date".into(),
                    span: Some(call_span),
                    help: None,
                    inner: vec![],
                })
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
        end_date = match Duration::try_days(days_to_output)
            .and_then(|days| start_date.checked_add_signed(days))
        {
            Some(date) => date,
            None => {
                return Err(ShellError::GenericError {
                    error: "int value too large".into(),
                    msg: "int value too large".into(),
                    span: Some(call_span),
                    help: None,
                    inner: vec![],
                });
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

    let Some(step_size) = Duration::try_days(step_size) else {
        return Err(ShellError::GenericError {
            error: "increment magnitude is too large".into(),
            msg: "increment magnitude is too large".into(),
            span: Some(call_span),
            help: None,
            inner: vec![],
        });
    };

    let mut next = start_date;
    if is_out_of_range(next) {
        return Err(ShellError::GenericError {
            error: "date is out of range".into(),
            msg: "date is out of range".into(),
            span: Some(call_span),
            help: None,
            inner: vec![],
        });
    }

    let mut ret = vec![];
    loop {
        let mut date_string = String::new();
        match write!(date_string, "{}", next.format(&out_format)) {
            Ok(_) => {}
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "Invalid output format".into(),
                    msg: e.to_string(),
                    span: Some(call_span),
                    help: None,
                    inner: vec![],
                });
            }
        }
        ret.push(Value::string(date_string, call_span));
        if let Some(n) = next.checked_add_signed(step_size) {
            next = n;
        } else {
            return Err(ShellError::GenericError {
                error: "date overflow".into(),
                msg: "adding the increment overflowed".into(),
                span: Some(call_span),
                help: None,
                inner: vec![],
            });
        }

        if is_out_of_range(next) {
            break;
        }
    }

    Ok(Value::list(ret, call_span))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SeqDate {})
    }
}
