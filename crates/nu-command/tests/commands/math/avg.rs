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
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[100kb, 10b, 100mib] | math avg | to json -r"
    );

    assert_eq!(actual.out, "34985870");
}
