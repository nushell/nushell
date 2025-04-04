use data_encoding::HEXUPPER;
use rand::prelude::*;
use rand::random_range;
use rand_chacha::ChaCha8Rng;

use nu_test_support::nu;

mod base32;
mod base32hex;
mod base64;
mod hex;

/// Generate a few random binaries.
fn random_bytes() -> Vec<String> {
    const NUM: usize = 32;
    let mut rng = ChaCha8Rng::seed_from_u64(4);

    (0..NUM)
        .map(|_| {
            let length = random_range(0..512);
            let mut bytes = vec![0u8; length];
            rng.fill_bytes(&mut bytes);
            HEXUPPER.encode(&bytes)
        })
        .collect()
}

pub fn test_canonical(cmd: &str) {
    for value in random_bytes() {
        let outcome = nu!("0x[{}] | encode {1} | decode {1} | to nuon", value, cmd);
        let nuon_value = format!("0x[{value}]");
        assert_eq!(outcome.out, nuon_value);
    }
}

pub fn test_const(cmd: &str) {
    for value in random_bytes() {
        let outcome = nu!(
            r#"const out = (0x[{}] | encode {1} | decode {1} | encode hex); $out"#,
            value,
            cmd
        );
        assert_eq!(outcome.out, value);
    }
}
