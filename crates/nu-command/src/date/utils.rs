use chrono::{DateTime, FixedOffset, Local, LocalResult, TimeZone};
use nu_protocol::{ShellError, Span, Value};

pub fn unsupported_input_error(span: Span) -> Value {
    Value::Error {
        error: ShellError::UnsupportedInput(
            String::from(
                "Unable to parse into datetime.",
            ),
            span,
        ),
    }
}

pub(crate) fn parse_date_from_string(input: &str, span: Span) -> Result<DateTime<FixedOffset>, Value> {
    match dtparse::parse(input) {
        Ok((native_dt, fixed_offset)) => {
            let offset = match fixed_offset {
                Some(fo) => fo,
                None => *(Local::now().offset()),
            };
            match offset.from_local_datetime(&native_dt) {
                LocalResult::Single(d) => Ok(d),
                LocalResult::Ambiguous(d, _) => Ok(d),
                LocalResult::None => {
                    Err(Value::Error {
                        error: ShellError::CantConvert(
                            "could not convert to a timezone-aware datetime"
                                .to_string(),
                            "local time representation is invalid".to_string(),
                            span,
                        )
                    })
                }
            }
        }
        Err(_) => {
            Err(Value::Error {
                error: ShellError::UnsupportedInput(
                    "Cannot parse input string as datetime. Might be be using unsupported timezone or offset.".to_string(),
                    span,
                ),
            })
        }
    }
}
