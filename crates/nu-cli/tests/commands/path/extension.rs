use nu_test_support::{nu, pipeline};

#[test]
fn returns_extension_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "bacon." | path extension
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_extension_with_dot_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "bacon." | path extension -r .egg
        "#
    ));

    assert_eq!(actual.out, "bacon..egg");
}

#[test]
fn replaces_extension_of_empty_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "" | path extension -r egg
        "#
    ));

    assert_eq!(actual.out, "");
}
