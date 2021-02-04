use nu_test_support::{nu, pipeline};

#[test]
fn md_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[]; []] | from json | to md
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
            echo [[name]; [jason]] | to md
        "#
    ));

    assert_eq!(actual.out, "|name||-||jason|");
}

#[test]
fn md_table_pretty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[name]; [joseph]] | to md -p
        "#
    ));

    assert_eq!(actual.out, "| name   || ------ || joseph |");
}

#[test]
fn md_combined() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def title [] {
                echo [[H1]; ["Nu top meals"]]
            };

            def meals [] {
                echo [[dish]; [Arepa] [Taco] [Pizza]]
            };

            title
            | append $(meals)
            | to md --per-element --pretty
        "#
    ));

    assert_eq!(
        actual.out,
        "# Nu top meals| dish  || ----- || Arepa || Taco  || Pizza |"
    );
}
