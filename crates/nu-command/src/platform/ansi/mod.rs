mod ansi_;
mod gradient;
mod link;
mod strip;

pub use ansi_::AnsiCommand as Ansi;
pub use gradient::SubCommand as AnsiGradient;
pub use link::SubCommand as AnsiLink;
pub use strip::SubCommand as AnsiStrip;
