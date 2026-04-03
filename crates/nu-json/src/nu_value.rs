use nu_protocol::{Span, IntoValue};

use crate::Value as JsonValue;
use nu_protocol::Value as NuValue;

impl IntoValue for JsonValue {
    fn into_value(self, span: Span) -> NuValue {
        match self {
            JsonValue::String(s) => NuValue::string(s, span),
            JsonValue::Bool(b) => NuValue::bool(b, span),
            JsonValue::F64(f) => NuValue::float(f, span),
            JsonValue::I64(i) => NuValue::int(i, span),
            JsonValue::Null => NuValue::nothing(span),
            JsonValue::U64(u) => match i64::try_from(u) {
                Ok(i) => NuValue::int(i, span),
                Err(_) => NuValue::float(u as f64, span),
            },
            JsonValue::Array(array) => NuValue::list(
                array
                    .into_iter()
                    .map(|x| x.into_value(span))
                    .collect(),
                span,
            ),
            JsonValue::Object(k) => NuValue::record(
                k.into_iter()
                    .map(|(k, v)| (k, v.into_value(span)))
                    .collect(),
                span,
            ),
        }
    }
}
