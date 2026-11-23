use nu_protocol::{PipelineData, ShellError, Span, Value};

mod cos;
mod cosh;
mod sin;
mod sinh;
mod tan;
mod tanh;

mod exp;
mod ln;

mod arccos;
mod arccosh;
mod arcsin;
mod arcsinh;
mod arctan;
mod arctanh;

pub use cos::MathCos;
pub use cosh::MathCosH;
pub use sin::MathSin;
pub use sinh::MathSinH;
pub use tan::MathTan;
pub use tanh::MathTanH;

pub use exp::MathExp;
pub use ln::MathLn;

pub use arccos::MathArcCos;
pub use arccosh::MathArcCosH;
pub use arcsin::MathArcSin;
pub use arcsinh::MathArcSinH;
pub use arctan::MathArcTan;
pub use arctanh::MathArcTanH;

/// Reject an unbounded range, matching the element-wise math commands in nu-command.
pub(crate) fn ensure_bounded_range(input: &PipelineData, head: Span) -> Result<(), ShellError> {
    if let PipelineData::Value(v @ Value::Range { val, .. }, ..) = input
        && !val.is_bounded()
    {
        return Err(ShellError::IncorrectValue {
            msg: "Range must be bounded".into(),
            val_span: v.span(),
            call_span: head,
        });
    }
    Ok(())
}
