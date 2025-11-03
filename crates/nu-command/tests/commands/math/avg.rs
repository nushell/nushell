use nu_test_support::nu;

#[test]
fn can_average_numbers() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
         open sgml_description.json
         | get glossary.GlossDiv.GlossList.GlossEntry.Sections
         | math avg
     "#);

    assert_eq!(actual.out, "101.5")
}

#[test]
fn can_average_bytes() {
    let actual = nu!("[100kb, 10b, 100mib] | math avg | to json -r");

    assert_eq!(actual.out, "34985870");
}

#[test]
fn can_average_range() {
    let actual = nu!("0..5 | math avg");

    assert_eq!(actual.out, "2.5");
}

#[test]
fn cannot_average_infinite_range() {
    let actual = nu!("0.. | math avg");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}

#[test]
fn const_avg() {
    let actual = nu!("const AVG = [1 3 5] | math avg; $AVG");
    assert_eq!(actual.out, "3.0");
}
