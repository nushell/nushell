mod capitalize;
mod case;
mod collect;
mod contains;
mod downcase;

pub use capitalize::SubCommand as StrCapitalize;
pub use case::*;
pub use collect::*;
pub use contains::SubCommand as StrContains;
pub use downcase::SubCommand as StrDowncase;
