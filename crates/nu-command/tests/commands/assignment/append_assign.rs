use nu_test_support::{nu, pipeline};

#[test]
fn append_assign_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [1 2];
            $a ++= [3 4];
            $a
        "#
    ));

    let expected = nu!(
        cwd: ".", pipeline(
        r#"
            [1 2 3 4]
        "#
    ));

    print!("{}", actual.out);
    print!("{}", expected.out);
    assert_eq!(actual.out, expected.out);
}

#[test]
fn append_assign_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [a b];
            $a ++= [c d];
            $a
        "#
    ));

    let expected = nu!(
        cwd: ".", pipeline(
        r#"
            [a b c d]
        "#
    ));

    print!("{}", actual.out);
    print!("{}", expected.out);
    assert_eq!(actual.out, expected.out);
}

#[test]
fn append_assign_any() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [1 2 a];
            $a ++= [b 3];
            $a
        "#
    ));

    let expected = nu!(
        cwd: ".", pipeline(
        r#"
            [1 2 a b 3]
        "#
    ));

    print!("{}", actual.out);
    print!("{}", expected.out);
    assert_eq!(actual.out, expected.out);
}

#[test]
fn append_assign_both_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [];
            $a ++= [];
            $a
        "#
    ));

    let expected = nu!(
        cwd: ".", pipeline(
        r#"
            []
        "#
    ));

    print!("{}", actual.out);
    print!("{}", expected.out);
    assert_eq!(actual.out, expected.out);
}

#[test]
fn append_assign_type_mismatch() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [1 2];
            $a ++= [a];
        "#
    ));

    assert!(actual
        .err
        .contains("expected list<int>, found list<string>"));
}
