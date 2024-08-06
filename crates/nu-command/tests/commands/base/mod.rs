use data_encoding::HEXUPPER;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

mod base32;
mod base32hex;
mod base64;
mod hex;

/// Generate a few random binaries.
pub fn random_bytes() -> Vec<String> {
    const NUM: usize = 32;
    let mut rng = ChaCha8Rng::seed_from_u64(4);

    (0..NUM)
        .map(|_| {
            let length = rng.gen_range(0..512);
            let mut bytes = vec![0u8; length];
            rng.fill_bytes(&mut bytes);
            let hex_bytes = HEXUPPER.encode(&bytes);
            format!("0x[{}]", hex_bytes)
        })
        .collect()
}
