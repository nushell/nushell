use data_encoding::HEXUPPER;
use rand::prelude::*;
use rand::random_range;
use rand_chacha::ChaCha8Rng;

use nu_test_support::prelude::*;

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

pub fn test_canonical(cmd: &str) -> Result {
    let mut tester = test();
    for value in random_bytes() {
        let code = format!("0x[{value}] | encode {cmd} | decode {cmd} | to nuon");
        let outcome: String = tester.run(code)?;
        let nuon_value = format!("0x[{value}]");
        assert_eq!(outcome, nuon_value);
    }
    Ok(())
}

pub fn test_const(cmd: &str) -> Result {
    let mut tester = test();
    for value in random_bytes() {
        let code = format!(
            r#"const out = (0x[{value}] | encode {cmd} | decode {cmd} | encode hex); $out"#
        );
        let outcome: String = tester.run(code)?;
        assert_eq!(outcome, value);
    }
    Ok(())
}
