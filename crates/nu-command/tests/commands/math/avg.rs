use nu_test_support::{nu, pipeline};

#[test]
fn can_average_numbers() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
             open sgml_description.json
             | get glossary.GlossDiv.GlossList.GlossEntry.Sections
             | math avg
         "#
    ));

    assert_eq!(actual.out, "101.5")
}

#[test]
fn can_average_bytes() {
    let actual = nu!("[100kb, 10b, 100mib] | math avg | to json -r");

    assert_eq!(actual.out, "34985870");
}

#[test]
fn const_avg() {
    let actual = nu!("const AVG = [1 3 5] | math avg; $AVG");
    assert_eq!(actual.out, "3");
}
