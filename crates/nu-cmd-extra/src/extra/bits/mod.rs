pub(crate) mod and;
pub(crate) mod bits_;
pub(crate) mod not;
pub(crate) mod or;
pub(crate) mod rotate_left;
pub(crate) mod rotate_right;
pub(crate) mod shift_left;
pub(crate) mod shift_right;
pub(crate) mod xor;

use nu_protocol::Spanned;

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

fn get_number_bytes(number_bytes: &Option<Spanned<String>>) -> NumberBytes {
    match number_bytes.as_ref() {
        None => NumberBytes::Eight,
        Some(size) => match size.item.as_str() {
            "1" => NumberBytes::One,
            "2" => NumberBytes::Two,
            "4" => NumberBytes::Four,
            "8" => NumberBytes::Eight,
            "auto" => NumberBytes::Auto,
            _ => NumberBytes::Invalid,
        },
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
