mod bits;
mod command;
mod number;

pub use command::FormatPattern;
// TODO remove `format_bits` visibility after removal of into bits
pub use bits::{format_bits, FormatBits};
// TODO remove `format_number` visibility after removal of into bits
pub use number::{format_number, FormatNumber};
