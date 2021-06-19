mod command;
mod gradient;
mod strip;

pub use command::Command as Ansi;
pub use gradient::SubCommand as AnsiGradient;
pub use strip::SubCommand as AnsiStrip;
