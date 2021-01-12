pub mod chars;
pub mod column;
pub mod command;
pub mod row;

pub use chars::SubCommand as SplitChars;
pub use column::SubCommand as SplitColumn;
pub use command::Command as Split;
pub use row::SubCommand as SplitRow;
