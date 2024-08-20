mod binary;
mod bool;
mod chars;
mod dice;
mod float;
mod int;
mod random_;
mod uuid;

pub use self::binary::SubCommand as RandomBinary;
pub use self::bool::SubCommand as RandomBool;
pub use self::chars::SubCommand as RandomChars;
pub use self::dice::SubCommand as RandomDice;
pub use self::float::SubCommand as RandomFloat;
pub use self::int::SubCommand as RandomInt;
pub use self::uuid::SubCommand as RandomUuid;
pub use random_::RandomCommand as Random;

use nu_engine::command_prelude::*;
use rand::{thread_rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

// Get the RNG generator for a subcommand.
pub fn rng(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Box<dyn RngCore>, ShellError> {
    if let Some(seed) = call.get_flag::<i64>(engine_state, stack, "seed")? {
        // The exact semantics are not important.  The only requirement for
        // this conversion is that it stays reproducible between versions
        // and platforms.
        let seed = u64::from_le_bytes(seed.to_le_bytes());
        Ok(Box::new(ChaCha8Rng::seed_from_u64(seed)))
    } else {
        Ok(Box::new(thread_rng()))
    }
}
