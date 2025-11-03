use nu_test_support::nu;

#[test]
fn canonical() {
    super::test_canonical("base32hex");
    super::test_canonical("base32hex --nopad");
}

#[test]
fn const_() {
    super::test_const("base32hex");
    super::test_const("base32hex --nopad");
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "AF685J4FPIJCP5UDJ5NSR5UCHJ6OLJ5IPIPCP5RDPI5CP4UCG76O5J4TCN6O9J4CPM2CP06CKR6A3J4UPII21J4EPIVSR1ECKN69RJ5ACR6PFJC4PI6SP1MCLB6BNJ57PM4NBJCCPM2CPBMCJN6OARMDGJ68LJ5PPIF6NJCOPI1CPBMDGV69VJCEF76BTJ4LPI3CR1ECMB6AFJ5043685J4GPICSR5MCLN6P8T6CG76PHJ41PIF6BJ5TPIHCR5ECL1SCR2UCKLQCP4ECIJ6PRJCD";

    let outcome = nu!(format!("'{text}' | encode base32hex --nopad"));
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "APIN4U90D5MN0RRIEHGMST10CHGN8O8=";

    let outcome = nu!(format!("'{encoded}' | decode base32hex | decode"));
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_pad_nopad() {
    let text = "®lnnE¾ˆë";
    let encoded_pad = "OAN6ORJE8N1BTIS6OELG====";
    let encoded_nopad = "OAN6ORJE8N1BTIS6OELG";

    let outcome = nu!(format!("'{encoded_pad}' | decode base32hex | decode"));
    assert_eq!(outcome.out, text);

    let outcome = nu!(format!(
        "'{encoded_nopad}' | decode base32hex --nopad | decode"
    ));
    assert_eq!(outcome.out, text);
}

#[test]
fn reject_pad_nopad() {
    let encoded_nopad = "D1KG";
    let encoded_pad = "D1KG====";

    let outcome = nu!(format!("'{encoded_nopad}' | decode base32hex"));
    assert!(!outcome.err.is_empty());

    let outcome = nu!(format!("'{encoded_pad}' | decode base32hex --nopad"));
    assert!(!outcome.err.is_empty())
}
