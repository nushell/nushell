use nu_test_support::{nu, pipeline};

#[test]
fn insert_plugin() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.newdep "1"
            | get dev-dependencies.newdep
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn downcase_upcase() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo abcd | wrap downcase | insert upcase { echo $it.downcase | str upcase } | format "{downcase}{upcase}" 
        "#
    ));

    assert_eq!(actual.out, "abcdABCD");
}

#[test]
fn number_and_its_negative_equal_zero() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1..10 | wrap num | insert neg { = $it.num * -1 } | math sum | = $it.num + $it.neg
        "#
    ));

    assert_eq!(actual.out, "0");
}
