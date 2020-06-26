pub mod bool;
pub mod command;
pub mod uuid;

pub use self::bool::SubCommand as RandomBool;
pub use command::Command as Random;
pub use uuid::SubCommand as RandomUUID;
