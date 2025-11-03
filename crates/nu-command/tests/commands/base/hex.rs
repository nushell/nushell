use nu_test_support::nu;

#[test]
fn canonical() {
    super::test_canonical("hex");
}

#[test]
fn const_() {
    super::test_const("hex");
}

#[test]
fn encode() {
    let text = "Ș̗͙̂̏o̲̲̗͗̌͊m̝̊̓́͂ë̡̦̞̤́̌̈́̀ ̥̝̪̎̿ͅf̧̪̻͉͗̈́̍̆u̮̝͌̈́ͅn̹̞̈́̊k̮͇̟͎̂͘y̧̲̠̾̆̕ͅ ̙͖̭͔̂̐t̞́́͘e̢̨͕̽x̥͋t͍̑̔͝";
    let encoded = "53CC82CC8FCCA6CC97CD996FCD97CC8CCD8ACCB2CCB2CC976DCC8ACC93CC81CD82CC9D65CD84CC8CCD84CC80CCA6CCA1CC9ECCA420CC8ECCBFCD85CCA5CC9DCCAA66CD97CD84CC8DCC86CCAACCBBCCA7CD8975CD8CCD84CCAECC9DCD856ECD84CC8ACCB9CC9E6BCD98CC82CCAECD87CC9FCD8E79CCBECC95CC86CD85CCB2CCA7CCA020CC82CC90CC99CD96CCADCD9474CC81CD98CC81CC9E65CCBDCCA2CD95CCA878CD8BCCA574CC91CC94CD9DCD8D";

    let outcome = nu!(format!("'{text}' | encode hex"));
    assert_eq!(outcome.out, encoded);
}

#[test]
fn decode_string() {
    let text = "Very important data";
    let encoded = "5665727920696D706F7274616E742064617461";

    let outcome = nu!(format!("'{encoded}' | decode hex | decode"));
    assert_eq!(outcome.out, text);
}

#[test]
fn decode_case_mixing() {
    let text = "®lnnE¾ˆë";
    let mixed_encoded = "c2aE6c6e6E45C2BeCB86c3ab";

    let outcome = nu!(format!("'{mixed_encoded}' | decode hex | decode"));
    assert_eq!(outcome.out, text);
}
