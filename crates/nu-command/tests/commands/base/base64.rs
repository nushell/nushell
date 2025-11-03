use nu_test_support::nu;

#[test]
fn canonical() {
    super::test_canonical("base64");
    super::test_canonical("base64 --url");
    super::test_canonical("base64 --nopad");
    super::test_canonical("base64 --url --nopad");
}

#[test]
fn const_() {
    super::test_const("base64");
    super::test_const("base64 --url");
    super::test_const("base64 --nopad");
    super::test_const("base64 --url --nopad");
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "U8yCzI/MpsyXzZlvzZfMjM2KzLLMssyXbcyKzJPMgc2CzJ1lzYTMjM2EzIDMpsyhzJ7MpCDMjsy/zYXMpcydzKpmzZfNhMyNzIbMqsy7zKfNiXXNjM2EzK7Mnc2Fbs2EzIrMucyea82YzILMrs2HzJ/NjnnMvsyVzIbNhcyyzKfMoCDMgsyQzJnNlsytzZR0zIHNmMyBzJ5lzL3Mos2VzKh4zYvMpXTMkcyUzZ3NjQ==";

    let outcome = nu!(format!("'{text}' | encode base64"));
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "VmVyeSBpbXBvcnRhbnQgZGF0YQ==";

    let outcome = nu!(format!("'{encoded}' | decode base64 | decode"));
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_pad_nopad() {
    let text = "”¥.ä@°bZö¢";
    let encoded_pad = "4oCdwqUuw6RAwrBiWsO2wqI=";
    let encoded_nopad = "4oCdwqUuw6RAwrBiWsO2wqI";

    let outcome = nu!(format!("'{encoded_pad}' | decode base64 | decode"));
    assert_eq!(outcome.out, text);

    let outcome = nu!(format!(
        "'{encoded_nopad}' | decode base64 --nopad | decode"
    ));
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_url() {
    let text = "p:gטݾ߫t+?";
    let encoded = "cDpn15jdvt+rdCs/";
    let encoded_url = "cDpn15jdvt-rdCs_";

    let outcome = nu!(format!("'{encoded}' | decode base64 | decode"));
    assert_eq!(outcome.out, text);

    let outcome = nu!(format!("'{encoded_url}' | decode base64 --url | decode"));
    assert_eq!(outcome.out, text);
}

#[test]
fn reject_pad_nopad() {
    let encoded_nopad = "YQ";
    let encoded_pad = "YQ==";

    let outcome = nu!(format!("'{encoded_nopad}' | decode base64"));
    assert!(!outcome.err.is_empty());

    let outcome = nu!(format!("'{encoded_pad}' | decode base64 --nopad"));
    assert!(!outcome.err.is_empty())
}
