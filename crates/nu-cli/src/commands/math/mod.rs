pub mod average;
pub mod command;
pub mod max;
pub mod min;
pub mod utils;

pub use average::SubCommand as Average;
pub use command::Command as Math;
pub use max::SubCommand as Maximum;
pub use min::SubCommand as Minimum;
