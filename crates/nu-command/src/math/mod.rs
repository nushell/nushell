mod abs;
mod avg;
pub mod command;
mod reducers;
mod utils;

pub use abs::SubCommand as MathAbs;
pub use avg::SubCommand as MathAvg;
pub use command::MathCommand as Math;
