use nu_protocol::Value;

use nu_protocol::Span;

/// Accumulates numbers into either an int or a float. Changes type to float on the first
/// float received.
#[derive(Clone, Copy)]
pub(crate) enum IntOrFloat {
    Int(i64),
    Float(f64),
}

impl IntOrFloat {
    pub(crate) fn add_i64(&mut self, n: i64) {
        match self {
            IntOrFloat::Int(ref mut v) => {
                *v += n;
            }
            IntOrFloat::Float(ref mut v) => {
                *v += n as f64;
            }
        }
    }

    pub(crate) fn add_f64(&mut self, n: f64) {
        match self {
            IntOrFloat::Int(v) => {
                *self = IntOrFloat::Float(*v as f64 + n);
            }
            IntOrFloat::Float(ref mut v) => {
                *v += n;
            }
        }
    }

    pub(crate) fn to_value(self, span: Span) -> Value {
        match self {
            IntOrFloat::Int(v) => Value::int(v, span),
            IntOrFloat::Float(v) => Value::float(v, span),
        }
    }
}
