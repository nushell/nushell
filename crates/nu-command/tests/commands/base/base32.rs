use nu_test_support::nu;

#[test]
fn canonical() {
    for value in super::random_bytes() {
        let outcome = nu!("{} | encode base32 | decode base32 | to nuon", value);
        assert_eq!(outcome.out, value);

        let outcome = nu!(
            "{} | encode base32 --nopad | decode base32 --nopad | to nuon",
            value
        );
        assert_eq!(outcome.out, value);
    }
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "KPGIFTEPZSTMZF6NTFX43F6MRTGYVTFSZSZMZF3NZSFMZE6MQHGYFTE5MXGYJTEMZWCMZAGMU3GKDTE6ZSSCBTEOZS743BOMUXGJ3TFKM3GZPTMEZSG4ZBWMVLGLXTFHZWEXLTMMZWCMZLWMTXGYK3WNQTGIVTFZZSPGXTMYZSBMZLWNQ7GJ7TMOPHGL5TEVZSDM3BOMWLGKPTFAEDGIFTEQZSM43FWMVXGZI5GMQHGZRTEBZSPGLTF5ZSRM3FOMVB4M3C6MUV2MZEOMSTGZ3TMN";

    let outcome = nu!("'{}' | encode base32 --nopad", text);
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "KZSXE6JANFWXA33SORQW45BAMRQXIYI=";

    let outcome = nu!("'{}' | decode base32 | decode", encoded);
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_pad_nopad() {
    let text = "®lnnE¾ˆë";
    let encoded_pad = "YKXGY3TOIXBL5S4GYOVQ====";
    let encoded_nopad = "YKXGY3TOIXBL5S4GYOVQ";

    let outcome = nu!("'{}' | decode base32 | decode", encoded_pad);
    assert_eq!(outcome.out, text);

    let outcome = nu!("'{}' | decode base32 --nopad | decode", encoded_nopad);
    assert_eq!(outcome.out, text);
}

#[test]
fn reject_pad_nopad() {
    let encoded_nopad = "ME";
    let encoded_pad = "ME======";

    let outcome = nu!("'{}' | decode base32", encoded_nopad);
    assert!(!outcome.err.is_empty());

    let outcome = nu!("'{}' | decode base32 --nopad", encoded_pad);
    assert!(!outcome.err.is_empty())
}
