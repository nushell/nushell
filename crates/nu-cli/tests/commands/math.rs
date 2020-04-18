use nu_test_support::{nu, pipeline};

#[test]
fn one_arg() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1
        "#
    ));

    assert_eq!(actual, "1");
}

#[test]
fn add() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 1
        "#
    ));

    assert_eq!(actual, "2");
}

#[test]
fn add_compount() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 + 2
        "#
    ));

    assert_eq!(actual, "5");
}

#[test]
fn precedence_of_operators() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 * 2
        "#
    ));

    assert_eq!(actual, "5");
}

#[test]
fn precedence_of_operators2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 * 2 + 1
        "#
    ));

    assert_eq!(actual, "6");
}

#[test]
fn division_of_ints() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 / 2
        "#
    ));

    assert_eq!(actual, "2");
}

#[test]
fn division_of_ints2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 / 4
        "#
    ));

    assert_eq!(actual, "0.25");
}

#[test]
fn parens_precedence() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 * (6 - 3)
        "#
    ));

    assert_eq!(actual, "12");
}

#[test]
fn compound_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 > 3 && 2 > 1
        "#
    ));

    assert_eq!(actual, "true");
}

#[test]
fn compound_comparison2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 < 3 || 2 > 1
        "#
    ));

    assert_eq!(actual, "true");
}

#[test]
fn compound_where() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from-json | where a == 2 && b == 1 | to-json
        "#
    ));

    assert_eq!(actual, r#"{"a":2,"b":1}"#);
}
