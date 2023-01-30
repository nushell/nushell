use nu_test_support::{nu, pipeline};

#[test]
fn each_works_separately() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3] | each { |it| echo $it 10 | math sum } | to json -r
        "#
    ));

    assert_eq!(actual.out, "[11,12,13]");
}

#[test]
fn each_group_works() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4 5 6] | group 3 | to json --raw
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[4,5,6]]");
}

#[test]
fn each_window() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4] | window 3 | to json --raw
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[2,3,4]]");
}

#[test]
fn each_window_stride() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4 5 6] | window 3 -s 2 | to json --raw
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[3,4,5]]");
}

#[test]
fn each_no_args_in_block() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [[foo bar]; [a b] [c d] [e f]] | each {|i| $i | to json -r } | get 1
        "#
    ));

    assert_eq!(actual.out, r#"{"foo": "c","bar": "d"}"#);
}

#[test]
fn each_implicit_it_in_block() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [[foo bar]; [a b] [c d] [e f]] | each { |it| nu --testbin cococo $it.foo } | str join
        "#
    ));

    assert_eq!(actual.out, "ace");
}
