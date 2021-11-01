mod abs;
mod avg;
pub mod command;
mod max;
mod min;
mod reducers;
mod utils;

pub use abs::SubCommand as MathAbs;
pub use avg::SubCommand as MathAvg;
pub use command::MathCommand as Math;
pub use max::SubCommand as MathMax;
pub use min::SubCommand as MathMin;
