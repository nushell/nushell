use nu_test_support::prelude::*;

#[test]
fn canonical() -> Result {
    super::test_canonical("base32")?;
    super::test_canonical("base32 --nopad")?;
    Ok(())
}

#[test]
fn const_() -> Result {
    super::test_const("base32")?;
    super::test_const("base32 --nopad")?;
    Ok(())
}

#[test]
fn encode() -> Result {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "KPGIFTEPZSTMZF6NTFX43F6MRTGYVTFSZSZMZF3NZSFMZE6MQHGYFTE5MXGYJTEMZWCMZAGMU3GKDTE6ZSSCBTEOZS743BOMUXGJ3TFKM3GZPTMEZSG4ZBWMVLGLXTFHZWEXLTMMZWCMZLWMTXGYK3WNQTGIVTFZZSPGXTMYZSBMZLWNQ7GJ7TMOPHGL5TEVZSDM3BOMWLGKPTFAEDGIFTEQZSM43FWMVXGZI5GMQHGZRTEBZSPGLTF5ZSRM3FOMVB4M3C6MUV2MZEOMSTGZ3TMN";

    test()
        .run_with_data("encode base32 --nopad", text)
        .expect_value_eq(encoded)
}

#[test]
fn decode_string() -> Result {
    let text = "Very important data";
    let encoded = "KZSXE6JANFWXA33SORQW45BAMRQXIYI=";

    test()
        .run_with_data("decode base32 | decode", encoded)
        .expect_value_eq(text)
}

#[test]
fn decode_pad_nopad() -> Result {
    let text = "®lnnE¾ˆë";
    let encoded_pad = "YKXGY3TOIXBL5S4GYOVQ====";
    let encoded_nopad = "YKXGY3TOIXBL5S4GYOVQ";

    test()
        .run_with_data("decode base32 | decode", encoded_pad)
        .expect_value_eq(text)?;

    test()
        .run_with_data("decode base32 --nopad | decode", encoded_nopad)
        .expect_value_eq(text)
}

#[test]
fn reject_pad_nopad() -> Result {
    let encoded_nopad = "ME";
    let encoded_pad = "ME======";

    let err = test()
        .run_with_data("decode base32", encoded_nopad)
        .expect_error()?;
    assert!(!err.to_string().is_empty());

    let err = test()
        .run_with_data("decode base32 --nopad", encoded_pad)
        .expect_error()?;
    assert!(!err.to_string().is_empty());
    Ok(())
}
