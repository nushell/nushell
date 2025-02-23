mod bits;
mod command;
mod number;

pub(crate) use command::FormatPattern;
// TODO remove `format_bits` visibility after removal of into bits
pub(crate) use bits::{format_bits, FormatBits};
// TODO remove `format_number` visibility after removal of into bits
pub(crate) use number::{format_number, FormatNumber};
