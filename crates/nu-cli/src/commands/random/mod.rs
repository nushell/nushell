pub mod command;

pub mod bool;
pub mod dice;
pub mod uuid;

pub use command::Command as Random;

pub use self::bool::SubCommand as RandomBool;
pub use dice::SubCommand as RandomDice;
pub use uuid::SubCommand as RandomUUID;
