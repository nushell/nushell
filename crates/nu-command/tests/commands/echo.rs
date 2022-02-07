use nu_test_support::{nu, pipeline};

#[test]
fn echo_range_is_lazy() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
<<<<<<< HEAD
        echo 1..10000000000 | first 3 | to json
=======
        echo 1..10000000000 | first 3 | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "[1,2,3]");
}

#[test]
fn echo_range_handles_inclusive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
<<<<<<< HEAD
        echo 1..3 | to json
=======
        echo 1..3 | each { $it } | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "[1,2,3]");
}

#[test]
fn echo_range_handles_exclusive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
<<<<<<< HEAD
        echo 1..<3 | to json
=======
        echo 1..<3 | each { $it } | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "[1,2]");
}

#[test]
fn echo_range_handles_inclusive_down() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
<<<<<<< HEAD
        echo 3..1 | to json
=======
        echo 3..1 | each { $it } | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "[3,2,1]");
}

#[test]
fn echo_range_handles_exclusive_down() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
<<<<<<< HEAD
        echo 3..<1 | to json
=======
        echo 3..<1 | each { $it } | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "[3,2]");
}
