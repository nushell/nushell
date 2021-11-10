mod capitalize;
mod case;
mod collect;
mod contains;
mod downcase;
mod ends_with;

pub use capitalize::SubCommand as StrCapitalize;
pub use case::*;
pub use collect::*;
pub use contains::SubCommand as StrContains;
pub use downcase::SubCommand as StrDowncase;
pub use ends_with::SubCommand as StrEndswith;
