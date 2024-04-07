mod and;
mod bits_;
mod into;
mod not;
mod or;
mod rotate_left;
mod rotate_right;
mod shift_left;
mod shift_right;
mod xor;

pub use and::BitsAnd;
pub use bits_::Bits;
pub use into::BitsInto;
pub use not::BitsNot;
pub use or::BitsOr;
pub use rotate_left::BitsRol;
pub use rotate_right::BitsRor;
pub use shift_left::BitsShl;
pub use shift_right::BitsShr;
pub use xor::BitsXor;

use nu_protocol::{ShellError, Span, Spanned, Value};
use std::iter;

#[derive(Clone, Copy)]
enum NumberBytes {
    One,
    Two,
    Four,
    Eight,
    Auto,
}

#[derive(Clone, Copy)]
enum InputNumType {
    One,
    Two,
    Four,
    Eight,
    SignedOne,
    SignedTwo,
    SignedFour,
    SignedEight,
}

fn get_number_bytes(
    number_bytes: Option<Spanned<usize>>,
    head: Span,
) -> Result<NumberBytes, ShellError> {
    match number_bytes {
        None => Ok(NumberBytes::Auto),
        Some(Spanned { item: 1, .. }) => Ok(NumberBytes::One),
        Some(Spanned { item: 2, .. }) => Ok(NumberBytes::Two),
        Some(Spanned { item: 4, .. }) => Ok(NumberBytes::Four),
        Some(Spanned { item: 8, .. }) => Ok(NumberBytes::Eight),
        Some(Spanned { span, .. }) => Err(ShellError::UnsupportedInput {
            msg: "Only 1, 2, 4, or 8 bytes are supported as word sizes".to_string(),
            input: "value originates from here".to_string(),
            msg_span: head,
            input_span: span,
        }),
    }
}

fn get_input_num_type(val: i64, signed: bool, number_size: NumberBytes) -> InputNumType {
    if signed || val < 0 {
        match number_size {
            NumberBytes::One => InputNumType::SignedOne,
            NumberBytes::Two => InputNumType::SignedTwo,
            NumberBytes::Four => InputNumType::SignedFour,
            NumberBytes::Eight => InputNumType::SignedEight,
            NumberBytes::Auto => {
                if val <= 0x7F && val >= -(2i64.pow(7)) {
                    InputNumType::SignedOne
                } else if val <= 0x7FFF && val >= -(2i64.pow(15)) {
                    InputNumType::SignedTwo
                } else if val <= 0x7FFFFFFF && val >= -(2i64.pow(31)) {
                    InputNumType::SignedFour
                } else {
                    InputNumType::SignedEight
                }
            }
        }
    } else {
        match number_size {
            NumberBytes::One => InputNumType::One,
            NumberBytes::Two => InputNumType::Two,
            NumberBytes::Four => InputNumType::Four,
            NumberBytes::Eight => InputNumType::Eight,
            NumberBytes::Auto => {
                if val <= 0xFF {
                    InputNumType::One
                } else if val <= 0xFFFF {
                    InputNumType::Two
                } else if val <= 0xFFFFFFFF {
                    InputNumType::Four
                } else {
                    InputNumType::Eight
                }
            }
        }
    }
}

fn binary_op<F>(lhs: &Value, rhs: &Value, little_endian: bool, f: F, head: Span) -> Value
where
    F: Fn((i64, i64)) -> i64,
{
    let span = lhs.span();
    match (lhs, rhs) {
        (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
            Value::int(f((*lhs, *rhs)), span)
        }
        (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
            let (lhs, rhs, max_len, min_len) = match (lhs.len(), rhs.len()) {
                (max, min) if max > min => (lhs, rhs, max, min),
                (min, max) => (rhs, lhs, max, min),
            };

            let pad = iter::repeat(0).take(max_len - min_len);

            let mut a;
            let mut b;

            let padded: &mut dyn Iterator<Item = u8> = if little_endian {
                a = rhs.iter().copied().chain(pad);
                &mut a
            } else {
                b = pad.chain(rhs.iter().copied());
                &mut b
            };

            let bytes: Vec<u8> = lhs
                .iter()
                .copied()
                .zip(padded)
                .map(|(lhs, rhs)| f((lhs as i64, rhs as i64)) as u8)
                .collect();

            Value::binary(bytes, span)
        }
        (Value::Binary { .. }, Value::Int { .. }) | (Value::Int { .. }, Value::Binary { .. }) => {
            Value::error(
                ShellError::PipelineMismatch {
                    exp_input_type: "input, and argument, to be both int or both binary"
                        .to_string(),
                    dst_span: rhs.span(),
                    src_span: span,
                },
                span,
            )
        }
        // Propagate errors by explicitly matching them before the final case.
        (e @ Value::Error { .. }, _) | (_, e @ Value::Error { .. }) => e.clone(),
        (other, Value::Int { .. } | Value::Binary { .. }) | (_, other) => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            span,
        ),
    }
}
