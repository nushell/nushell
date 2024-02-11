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

use nu_protocol::Value;
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
        _ => NumberBytes::Invalid
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
