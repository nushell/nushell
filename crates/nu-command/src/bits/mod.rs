mod and;
mod bits_;
mod not;
mod or;
mod shift_left;
mod shift_right;
mod xor;

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
}
