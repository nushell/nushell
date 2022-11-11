use chrono::naive::NaiveDate;
use chrono::{Duration, Local};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SeqDate;

impl Command for SeqDate {
    fn name(&self) -> &str {
        "seq date"
    }

    fn usage(&self) -> &str {
        "Print sequences of dates"
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
        let span = Span::test_data();

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
                result: Some(Value::List {
                    vals: vec![
                        Value::String { val: "2020-01-01".into(), span, },
                        Value::String { val: "2020-01-02".into(), span, },
                        Value::String { val: "2020-01-03".into(), span, },
                        Value::String { val: "2020-01-04".into(), span, },
                        Value::String { val: "2020-01-05".into(), span, },
                        Value::String { val: "2020-01-06".into(), span, },
                        Value::String { val: "2020-01-07".into(), span, },
                        Value::String { val: "2020-01-08".into(), span, },
                        Value::String { val: "2020-01-09".into(), span, },
                        Value::String { val: "2020-01-10".into(), span, },
                    ],
                    span,
                }),
            },
            Example {
                description: "print every fifth day between January 1st 2020 and January 31st 2020",
                example: "seq date -b '2020-01-01' -e '2020-01-31' -n 5",
                result: Some(Value::List {
                   vals: vec![
                    Value::String { val: "2020-01-01".into(), span, },
                    Value::String { val: "2020-01-06".into(), span, },
                    Value::String { val: "2020-01-11".into(), span, },
                    Value::String { val: "2020-01-16".into(), span, },
                    Value::String { val: "2020-01-21".into(), span, },
                    Value::String { val: "2020-01-26".into(), span, },
                    Value::String { val: "2020-01-31".into(), span, },
                    ],
                    span,
                }),
            },
            Example {
                description: "starting on May 5th, 2020, print the next 10 days in your locale's date format, colon separated",
                example: "seq date -o %x -s ':' -d 10 -b '2020-05-01'",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let output_format: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "output-format")?;
        let input_format: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "input-format")?;
        let begin_date: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "begin-date")?;
        let end_date: Option<Spanned<String>> = call.get_flag(engine_state, stack, "end-date")?;
        let increment: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "increment")?;
        let days: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "days")?;
        let reverse = call.has_flag("reverse");

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
            _ => Value::int(1_i64, Span::test_data()),
        };

        let day_count = days.map(|i| Value::int(i.item, i.span));

        let mut rev = false;
        if reverse {
            rev = reverse;
        }

        Ok(
            run_seq_dates(outformat, informat, begin, end, inc, day_count, rev)?
                .into_pipeline_data(),
        )
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
) -> Result<Value, ShellError> {
    let today = Local::today().naive_local();
    let mut step_size: i64 = increment
        .as_i64()
        .expect("unable to change increment to i64");

    if step_size == 0 {
        return Err(ShellError::GenericError(
            "increment cannot be 0".to_string(),
            "increment cannot be 0".to_string(),
            Some(increment.span()?),
            None,
            Vec::new(),
        ));
    }

    let in_format = match input_format {
        Some(i) => match i.as_string() {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "".to_string(),
                    None,
                    Some("error with input_format as_string".to_string()),
                    Vec::new(),
                ));
            }
        },
        _ => "%Y-%m-%d".to_string(),
    };

    let out_format = match output_format {
        Some(i) => match i.as_string() {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "".to_string(),
                    None,
                    Some("error with output_format as_string".to_string()),
                    Vec::new(),
                ));
            }
        },
        _ => "%Y-%m-%d".to_string(),
    };

    let start_date = match beginning_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "Failed to parse date".to_string(),
                    Some(Span::test_data()),
                    None,
                    Vec::new(),
                ))
            }
        },
        _ => today,
    };

    let mut end_date = match ending_date {
        Some(d) => match parse_date_string(&d, &in_format) {
            Ok(nd) => nd,
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "Failed to parse date".to_string(),
                    Some(Span::test_data()),
                    None,
                    Vec::new(),
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
                return Err(ShellError::GenericError(
                    "integer value too large".to_string(),
                    "integer value too large".to_string(),
                    Some(Span::test_data()),
                    None,
                    Vec::new(),
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
        return Err(ShellError::GenericError(
            "date is out of range".to_string(),
            "date is out of range".to_string(),
            Some(Span::test_data()),
            None,
            Vec::new(),
        ));
    }

    let mut ret = vec![];
    loop {
        let date_string = &next.format(&out_format).to_string();
        ret.push(Value::string(date_string, Span::test_data()));
        next += Duration::days(step_size);

        if is_out_of_range(next) {
            break;
        }
    }

    Ok(Value::List {
        vals: ret,
        span: Span::test_data(),
    })
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
