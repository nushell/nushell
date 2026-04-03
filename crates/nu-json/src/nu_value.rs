use nu_protocol::{ShellError, Span, TryIntoValue};

use crate::Value as JsonValue;
use nu_protocol::Value as NuValue;

impl TryIntoValue for JsonValue {
    fn try_into_value(self, span: Span) -> Result<NuValue, ShellError> {
        Ok(match self {
            JsonValue::String(s) => NuValue::string(s, span),
            JsonValue::Bool(b) => NuValue::bool(b, span),
            JsonValue::F64(f) => NuValue::float(f, span),
            JsonValue::I64(i) => NuValue::int(i, span),
            JsonValue::Null => NuValue::nothing(span),
            JsonValue::U64(u) => match i64::try_from(u) {
                Ok(i) => NuValue::int(i, span),
                Err(_) => {
                    return Err(ShellError::CantConvert {
                        to_type: "i64 sized integer".into(),
                        from_type: "value larger than i64".into(),
                        span,
                        help: None,
                    });
                }
            },
            JsonValue::Array(array) => NuValue::list(
                array
                    .into_iter()
                    .map(|x| x.try_into_value(span))
                    .collect::<Result<_, _>>()?,
                span,
            ),
            JsonValue::Object(k) => NuValue::record(
                k.into_iter()
                    .map(|(k, v)| v.try_into_value(span).map(|v| (k, v)))
                    .collect::<Result<_, _>>()?,
                span,
            ),
        })
    }
}
