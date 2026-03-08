use nu_test_support::prelude::*;

#[test]
fn canonical() -> Result {
    super::test_canonical("base64")?;
    super::test_canonical("base64 --url")?;
    super::test_canonical("base64 --nopad")?;
    super::test_canonical("base64 --url --nopad")?;
    Ok(())
}

#[test]
fn const_() -> Result {
    super::test_const("base64")?;
    super::test_const("base64 --url")?;
    super::test_const("base64 --nopad")?;
    super::test_const("base64 --url --nopad")?;
    Ok(())
}

#[test]
fn encode() -> Result {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "U8yCzI/MpsyXzZlvzZfMjM2KzLLMssyXbcyKzJPMgc2CzJ1lzYTMjM2EzIDMpsyhzJ7MpCDMjsy/zYXMpcydzKpmzZfNhMyNzIbMqsy7zKfNiXXNjM2EzK7Mnc2Fbs2EzIrMucyea82YzILMrs2HzJ/NjnnMvsyVzIbNhcyyzKfMoCDMgsyQzJnNlsytzZR0zIHNmMyBzJ5lzL3Mos2VzKh4zYvMpXTMkcyUzZ3NjQ==";

    let code = format!("'{text}' | encode base64");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, encoded);
    Ok(())
}

#[test]
fn decode_string() -> Result {
    let text = "Very important data";
    let encoded = "VmVyeSBpbXBvcnRhbnQgZGF0YQ==";

    let code = format!("'{encoded}' | decode base64 | decode");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, text);
    Ok(())
}

#[test]
fn decode_pad_nopad() -> Result {
    let text = "”¥.ä@°bZö¢";
    let encoded_pad = "4oCdwqUuw6RAwrBiWsO2wqI=";
    let encoded_nopad = "4oCdwqUuw6RAwrBiWsO2wqI";

    let code = format!("'{encoded_pad}' | decode base64 | decode");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, text);

    let code = format!("'{encoded_nopad}' | decode base64 --nopad | decode");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, text);
    Ok(())
}

#[test]
fn decode_url() -> Result {
    let text = "p:gטݾ߫t+?";
    let encoded = "cDpn15jdvt+rdCs/";
    let encoded_url = "cDpn15jdvt-rdCs_";

    let code = format!("'{encoded}' | decode base64 | decode");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, text);

    let code = format!("'{encoded_url}' | decode base64 --url | decode");
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, text);
    Ok(())
}

#[test]
fn reject_pad_nopad() -> Result {
    let encoded_nopad = "YQ";
    let encoded_pad = "YQ==";

    let code = format!("'{encoded_nopad}' | decode base64");
    let err = test().run(code).expect_error()?;
    assert!(!err.to_string().is_empty());

    let code = format!("'{encoded_pad}' | decode base64 --nopad");
    let err = test().run(code).expect_error()?;
    assert!(!err.to_string().is_empty());
    Ok(())
}
