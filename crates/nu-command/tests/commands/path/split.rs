use nu_test_support::{nu, pipeline};

#[test]
fn splits_empty_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo '' | path split
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn splits_correctly_with_column_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo [
                [home, barn];

                ['home/viking/spam.txt', 'barn/cow/moo.png']
                ['home/viking/eggs.txt', 'barn/goat/cheese.png']
            ]
            | path split home barn
            | get barn
            | length
        "#
    ));

    assert_eq!(actual.out, "6");
}
