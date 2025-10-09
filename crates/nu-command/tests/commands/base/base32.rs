use nu_test_support::nu;

#[test]
fn canonical() {
    super::test_canonical("base32");
    super::test_canonical("base32 --nopad");
}

#[test]
fn const_() {
    super::test_const("base32");
    super::test_const("base32 --nopad");
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "KPGIFTEPZSTMZF6NTFX43F6MRTGYVTFSZSZMZF3NZSFMZE6MQHGYFTE5MXGYJTEMZWCMZAGMU3GKDTE6ZSSCBTEOZS743BOMUXGJ3TFKM3GZPTMEZSG4ZBWMVLGLXTFHZWEXLTMMZWCMZLWMTXGYK3WNQTGIVTFZZSPGXTMYZSBMZLWNQ7GJ7TMOPHGL5TEVZSDM3BOMWLGKPTFAEDGIFTEQZSM43FWMVXGZI5GMQHGZRTEBZSPGLTF5ZSRM3FOMVB4M3C6MUV2MZEOMSTGZ3TMN";

    let outcome = nu!(format!("'{text}' | encode base32 --nopad"));
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "KZSXE6JANFWXA33SORQW45BAMRQXIYI=";

    let outcome = nu!(format!("'{encoded}' | decode base32 | decode"));
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_pad_nopad() {
    let text = "®lnnE¾ˆë";
    let encoded_pad = "YKXGY3TOIXBL5S4GYOVQ====";
    let encoded_nopad = "YKXGY3TOIXBL5S4GYOVQ";

    let outcome = nu!(format!("'{encoded_pad}' | decode base32 | decode"));
    assert_eq!(outcome.out, text);

    let outcome = nu!(format!(
        "'{encoded_nopad}' | decode base32 --nopad | decode"
    ));
    assert_eq!(outcome.out, text);
}

#[test]
fn reject_pad_nopad() {
    let encoded_nopad = "ME";
    let encoded_pad = "ME======";

    let outcome = nu!(format!("'{encoded_nopad}' | decode base32"));
    assert!(!outcome.err.is_empty());

    let outcome = nu!(format!("'{encoded_pad}' | decode base32 --nopad"));
    assert!(!outcome.err.is_empty())
}
