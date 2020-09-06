use nu_test_support::{nu, pipeline};

#[test]
fn each_works_separately() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3] | each { echo $it 10 | math sum } | to json | echo $it
        "#
    ));

    assert_eq!(actual.out, "[11,12,13]");
}

#[test]
fn each_group_works() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4 5 6] | each group 3 { echo $it } | to json
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[4,5,6]]");
}

#[test]
fn each_window() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4] | each window 3 { echo $it } | to json
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[2,3,4]]");
}

#[test]
fn each_window_stride() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3 4 5 6] | each window 3 -s 2 { echo $it } | to json
        "#
    ));

    assert_eq!(actual.out, "[[1,2,3],[3,4,5]]");
}
