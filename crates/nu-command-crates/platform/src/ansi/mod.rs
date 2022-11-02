mod ansi_;
mod gradient;
mod strip;

pub use ansi_::AnsiCommand as Ansi;
pub use gradient::SubCommand as AnsiGradient;
pub use strip::SubCommand as AnsiStrip;
