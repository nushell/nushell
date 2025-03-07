mod bits;
mod number;
mod pattern;

pub(crate) use pattern::FormatPattern;
// TODO remove `format_bits` visibility after removal of into bits
pub(crate) use bits::{format_bits, FormatBits};
// TODO remove `format_number` visibility after removal of into bits
pub(crate) use number::{format_number, FormatNumber};
