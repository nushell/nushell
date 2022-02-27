use chrono::{DateTime, FixedOffset, Local, LocalResult, TimeZone};
use nu_protocol::{ShellError, Span, Value};

pub(crate) fn parse_date_from_string(
    input: &str,
    span: Span,
) -> Result<DateTime<FixedOffset>, Value> {
    match dtparse::parse(input) {
        Ok((native_dt, fixed_offset)) => {
            let offset = match fixed_offset {
                Some(fo) => fo,
                None => *(Local::now().offset()),
            };
            match offset.from_local_datetime(&native_dt) {
                LocalResult::Single(d) => Ok(d),
                LocalResult::Ambiguous(d, _) => Ok(d),
                LocalResult::None => Err(Value::Error {
                    error: ShellError::DatetimeParseError(span),
                }),
            }
        }
        Err(_) => Err(Value::Error {
            error: ShellError::DatetimeParseError(span),
        }),
    }
}
