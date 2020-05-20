use nu_test_support::{nu, pipeline};

#[test]
fn creates_the_resulting_string_from_the_given_fields() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | get package
            | format "{name} has license {license}"
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "nu has license ISC");
}

#[test]
fn given_fields_can_be_column_paths() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | format "{package.name} is {package.description}"
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "nu is a new type of shell");
}

#[test]
fn can_use_variables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | format "{$it.package.name} is {$it.package.description}"
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "nu is a new type of shell");
}
