use nu_test_support::{nu, pipeline};

#[test]
fn pipe_assign_int_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [1 2];
            $a |= append [3 4];
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
fn pipe_assign_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            mut a = [1 -1];
            $a |= math abs | math sum;
            $a
        "#
    ));

    let expected = "2";

    print!("{}", actual.out);
    print!("{}", expected);
    assert_eq!(actual.out, expected);
}
