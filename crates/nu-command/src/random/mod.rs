mod bool;
mod chars;
mod dice;
mod float;
mod int;
mod random_;
mod uuid;

pub use random_::RandomCommand as Random;

pub use self::{
    bool::SubCommand as RandomBool, chars::SubCommand as RandomChars,
    dice::SubCommand as RandomDice, float::SubCommand as RandomFloat, int::SubCommand as RandomInt,
    uuid::SubCommand as RandomUuid,
};
