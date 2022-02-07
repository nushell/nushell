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
<<<<<<< HEAD
        "ls | sort-by name | skip 1 | first 2 | get size | math avg | format \"{$it}\" "
    );

    assert_eq!(actual.out, "1.6 KB");
=======
        "ls | sort-by name | skip 1 | first 2 | get size | math avg | to json -r"
    );

    assert_eq!(actual.out, "1600");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
