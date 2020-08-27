pub mod avg;
pub mod command;
pub mod eval;
pub mod max;
pub mod median;
pub mod min;
pub mod mode;
pub mod stddev;
pub mod sum;
pub mod variance;

mod reducers;
mod utils;

pub use avg::SubCommand as MathAverage;
pub use command::Command as Math;
pub use eval::SubCommand as MathEval;
pub use max::SubCommand as MathMaximum;
pub use median::SubCommand as MathMedian;
pub use min::SubCommand as MathMinimum;
pub use mode::SubCommand as MathMode;
pub use stddev::SubCommand as MathStddev;
pub use sum::SubCommand as MathSummation;
pub use variance::SubCommand as MathVariance;
