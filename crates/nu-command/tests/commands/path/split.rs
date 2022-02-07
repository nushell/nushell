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
fn splits_correctly_single_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
<<<<<<< HEAD
            echo ['home/viking/spam.txt']
=======
            'home/viking/spam.txt'
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            | path split
            | last
        "#
    ));

    assert_eq!(actual.out, "spam.txt");
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
            | path split -c [ home barn ]
            | get barn
<<<<<<< HEAD
=======
            | flatten
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            | length
        "#
    ));

    assert_eq!(actual.out, "6");
}
