pub mod average;
pub mod command;
pub mod max;
pub mod median;
pub mod min;
pub mod mode;
pub mod sum;
pub mod utils;

pub use average::SubCommand as MathAverage;
pub use command::Command as Math;
pub use max::SubCommand as MathMaximum;
pub use median::SubCommand as MathMedian;
pub use min::SubCommand as MathMinimum;
pub use mode::SubCommand as MathMode;
pub use sum::SubCommand as MathSummation;
