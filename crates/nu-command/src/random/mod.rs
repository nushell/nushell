mod bool;
mod chars;
mod command;
mod decimal;
mod dice;
mod integer;
mod uuid;

pub use self::bool::SubCommand as RandomBool;
pub use self::chars::SubCommand as RandomChars;
pub use self::decimal::SubCommand as RandomDecimal;
pub use self::dice::SubCommand as RandomDice;
pub use self::integer::SubCommand as RandomInteger;
pub use self::uuid::SubCommand as RandomUuid;
pub use command::RandomCommand as Random;
