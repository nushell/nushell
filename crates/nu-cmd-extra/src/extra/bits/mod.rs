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
use nu_protocol::{ShellError, Value};
pub use or::BitsOr;
pub use rotate_left::BitsRol;
pub use rotate_right::BitsRor;
pub use shift_left::BitsShl;
pub use shift_right::BitsShr;
pub use xor::BitsXor;

use std::iter;

#[derive(Clone, Copy)]
enum NumberBytes {
    One,
    Two,
    Four,
    Eight,
    Auto,
    Invalid,
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

fn get_number_bytes(number_bytes: Option<&Value>) -> NumberBytes {
    match number_bytes {
        None => NumberBytes::Eight,
        Some(Value::String { val, .. }) => match val.as_str() {
            "1" => NumberBytes::One,
            "2" => NumberBytes::Two,
            "4" => NumberBytes::Four,
            "8" => NumberBytes::Eight,
            "auto" => NumberBytes::Auto,
            _ => NumberBytes::Invalid,
        },
        Some(Value::Int { val, .. }) => match val {
            1 => NumberBytes::One,
            2 => NumberBytes::Two,
            4 => NumberBytes::Four,
            8 => NumberBytes::Eight,
            _ => NumberBytes::Invalid,
        },
        _ => NumberBytes::Invalid,
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
            NumberBytes::Invalid => InputNumType::SignedFour,
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
            NumberBytes::Invalid => InputNumType::Four,
        }
    }
}

fn binary_op<F>(lhs: &Value, rhs: &Value, little_endian: bool, f: F) -> Value
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
                (min, max) => (rhs, lhs, min, max),
            };

            let pad = iter::repeat(0).take(max_len - min_len);

            let mut a;
            let mut b;

            let padded: &mut dyn Iterator<Item = u8> = if little_endian {
                a = pad.chain(rhs.iter().copied());
                &mut a
            } else {
                b = rhs.iter().copied().chain(pad);
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
                dst_span: other.span(),
                src_span: span,
            },
            span,
        ),
    }
}
