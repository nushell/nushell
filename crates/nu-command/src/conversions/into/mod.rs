mod binary;
mod bool;
mod command;
mod datetime;
mod decimal;
mod duration;
mod filesize;
mod int;
mod string;

pub use self::bool::SubCommand as IntoBool;
pub use self::filesize::SubCommand as IntoFilesize;
pub use binary::SubCommand as IntoBinary;
pub use command::Into;
pub use datetime::SubCommand as IntoDatetime;
pub use decimal::SubCommand as IntoDecimal;
pub use duration::SubCommand as IntoDuration;
pub use int::SubCommand as IntoInt;
pub use string::SubCommand as IntoString;
