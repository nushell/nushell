mod bool;
mod chars;
mod dice;
mod float;
mod integer;
mod random_;
mod uuid;

pub use self::bool::SubCommand as RandomBool;
pub use self::chars::SubCommand as RandomChars;
pub use self::dice::SubCommand as RandomDice;
pub use self::float::SubCommand as RandomFloat;
pub use self::integer::SubCommand as RandomInteger;
pub use self::uuid::SubCommand as RandomUuid;
pub use random_::RandomCommand as Random;
