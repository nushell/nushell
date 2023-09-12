mod chars;
mod column;
mod command;
mod list;
mod words;

pub use chars::SubCommand as SplitChars;
pub use column::SubCommand as SplitColumn;
pub use command::SplitCommand as Split;
pub use list::SubCommand as SplitList;
pub use words::SubCommand as SplitWords;
