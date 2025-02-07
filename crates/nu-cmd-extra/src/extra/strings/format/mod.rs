mod bits;
mod command;
mod number;

pub(crate) use bits::FormatBits;
pub(crate) use command::FormatPattern;
// TODO remove `format_number` visibility after removal of into bits
pub(crate) use number::{format_number, FormatNumber};
