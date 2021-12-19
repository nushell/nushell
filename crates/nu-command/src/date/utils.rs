use chrono::{DateTime, FixedOffset};
use nu_protocol::{ShellError, Span, Value};

pub fn unsupported_input_error(span: Span) -> Value {
    Value::Error {
        error: ShellError::UnsupportedInput(
            String::from(
                "Only dates with timezones are supported. The following formats are allowed \n 
            * %Y-%m-%d %H:%M:%S %z -- 2020-04-12 22:10:57 +02:00 \n 
            * %Y-%m-%d %H:%M:%S%.6f %z -- 2020-04-12 22:10:57.213231 +02:00 \n 
            * rfc3339 -- 2020-04-12T22:10:57+02:00 \n 
            * rfc2822 -- Tue, 1 Jul 2003 10:52:37 +0200",
            ),
            span,
        ),
    }
}

pub fn parse_date_from_string(input: String, span: Span) -> Result<DateTime<FixedOffset>, Value> {
    let datetime = DateTime::parse_from_str(&input, "%Y-%m-%d %H:%M:%S %z"); // "2020-04-12 22:10:57 +02:00";
    match datetime {
        Ok(x) => Ok(x),
        Err(_) => {
            let datetime = DateTime::parse_from_str(&input, "%Y-%m-%d %H:%M:%S%.6f %z"); // "2020-04-12 22:10:57.213231 +02:00";
            match datetime {
                Ok(x) => Ok(x),
                Err(_) => {
                    let datetime = DateTime::parse_from_rfc3339(&input); // "2020-04-12T22:10:57+02:00";
                    match datetime {
                        Ok(x) => Ok(x),
                        Err(_) => {
                            let datetime = DateTime::parse_from_rfc2822(&input); // "Tue, 1 Jul 2003 10:52:37 +0200";
                            match datetime {
                                Ok(x) => Ok(x),
                                Err(_) => Err(unsupported_input_error(span)),
                            }
                        }
                    }
                }
            }
        }
    }
}
