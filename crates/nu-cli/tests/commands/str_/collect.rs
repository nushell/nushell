mod collect;

#[test]
fn test_1() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1..5 | str collect 
        "#
        )
    );

    assert_eq!(actual, "12345");
}

#[test]
fn test_2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [a b y z] | str collect $(char newline)
        "#
        )
    );

    assert_eq!(actual, "a\nb\ny\nz");
}

#[test]
fn construct_a_path() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [sample txt] | str collect "."
        "#
        )
    );

    assert_eq!(actual, "sample.txt");
}

fn sum_one_to_four() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1..4 | str collect "+" | math eval
        "#
        )
    );

    assert!(actual.contains("10.0"));
}
