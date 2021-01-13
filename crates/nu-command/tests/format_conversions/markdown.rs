use nu_test_support::{nu, pipeline};

#[test]
fn md_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo "{}" | from json | to md
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn md_empty_pretty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo "{}" | from json | to md -p
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn md_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to md
        "#
    ));

    assert_eq!(actual.out, "3");
}

#[test]
fn md_simple_pretty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to md -p
        "#
    ));

    assert_eq!(actual.out, "3");
}

#[test]
fn md_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "jason"}' | from json | to md
        "#
    ));

    assert_eq!(actual.out, "|name||-||jason|");
}

#[test]
fn md_table_pretty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "joseph"}' | from json | to md -p
        "#
    ));

    assert_eq!(actual.out, "| name   || ------ || joseph |");
}
