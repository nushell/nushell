mod and;
mod bits_;
mod not;
mod or;
mod shift_left;
mod shift_right;
mod xor;

use nu_protocol::Spanned;

pub use and::SubCommand as BitsAnd;
pub use bits_::Bits;
pub use not::SubCommand as BitsNot;
pub use or::SubCommand as BitsOr;
pub use shift_left::SubCommand as BitsShiftLeft;
pub use shift_right::SubCommand as BitsShiftRight;
pub use xor::SubCommand as BitsXor;

#[derive(Clone, Copy)]
enum NumberBytes {
    One,
    Two,
    Four,
    Eight,
    Auto,
    Invalid,
}

fn get_number_bytes(number_bytes: &Option<Spanned<String>>) -> NumberBytes {
    match number_bytes.as_ref() {
        None => NumberBytes::Auto,
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
