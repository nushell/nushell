mod bool;
mod chars;
mod command;
mod decimal;
mod dice;

pub use self::bool::SubCommand as Bool;
pub use self::chars::SubCommand as Chars;
pub use self::decimal::SubCommand as Decimal;
pub use self::dice::SubCommand as Dice;
pub use command::RandomCommand as Random;
