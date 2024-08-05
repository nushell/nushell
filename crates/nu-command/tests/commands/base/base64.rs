use nu_test_support::nu;

#[test]
fn canonical() {
    for value in super::random_bytes() {
        let outcome = nu!(
            "{} | encode new-base64 | decode new-base64 | to nuon",
            value
        );
        assert_eq!(outcome.out, value);

        let outcome = nu!(
            "{} | encode new-base64 --url | decode new-base64 --url | to nuon",
            value
        );
        assert_eq!(outcome.out, value);

        let outcome = nu!(
            "{} | encode new-base64 --nopad | decode new-base64 --nopad | to nuon",
            value
        );
        assert_eq!(outcome.out, value);

        let outcome = nu!(
            "{} | encode new-base64 --url --nopad | decode new-base64 --url --nopad | to nuon",
            value
        );
        assert_eq!(outcome.out, value);
    }
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "U8yCzI/MpsyXzZlvzZfMjM2KzLLMssyXbcyKzJPMgc2CzJ1lzYTMjM2EzIDMpsyhzJ7MpCDMjsy/zYXMpcydzKpmzZfNhMyNzIbMqsy7zKfNiXXNjM2EzK7Mnc2Fbs2EzIrMucyea82YzILMrs2HzJ/NjnnMvsyVzIbNhcyyzKfMoCDMgsyQzJnNlsytzZR0zIHNmMyBzJ5lzL3Mos2VzKh4zYvMpXTMkcyUzZ3NjQ==";

    let outcome = nu!("'{}' | encode new-base64", text);
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "VmVyeSBpbXBvcnRhbnQgZGF0YQ==";

    let outcome = nu!("'{}' | decode new-base64 | decode", encoded);
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_pad_nopad() {
    let text = "”¥.ä@°bZö¢";
    let encoded_pad = "4oCdwqUuw6RAwrBiWsO2wqI=";
    let encoded_nopad = "4oCdwqUuw6RAwrBiWsO2wqI";

    let outcome = nu!("'{}' | decode new-base64 | decode", encoded_pad);
    assert_eq!(outcome.out, text);

    let outcome = nu!("'{}' | decode new-base64 --nopad | decode", encoded_nopad);
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_url() {
    let text = "ޘ::߇3/Ծ]D";
    let encoded = "3pg6Ot+HMy/Uvl1E";
    let encoded_url = "3pg6Ot-HMy_Uvl1E";

    let outcome = nu!("'{}' | decode new-base64 | decode", encoded);
    assert_eq!(outcome.out, text);

    let outcome = nu!("'{}' | decode new-base64 --url | decode", encoded_url);
    assert_eq!(outcome.out, text);
}

#[test]
fn reject_pad_nopad() {
    let encoded_nopad = "YQ";
    let encoded_pad = "YQ==";

    let outcome = nu!("'{}' | decode new-base64", encoded_nopad);
    assert!(!outcome.err.is_empty());

    let outcome = nu!("'{}' | decode new-base64 --nopad", encoded_pad);
    assert!(!outcome.err.is_empty())
}
