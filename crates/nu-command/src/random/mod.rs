mod bool;
mod chars;
mod command;
mod decimal;

pub use self::bool::SubCommand as Bool;
pub use self::chars::SubCommand as Chars;
pub use self::decimal::SubCommand as Decimal;
pub use command::RandomCommand as Random;
