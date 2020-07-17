pub mod command;

pub mod bool;
pub mod dice;
#[cfg(feature = "uuid_crate")]
pub mod uuid;

pub use command::Command as Random;

pub use self::bool::SubCommand as RandomBool;
pub use dice::SubCommand as RandomDice;
#[cfg(feature = "uuid_crate")]
pub use uuid::SubCommand as RandomUUID;
