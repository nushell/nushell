use nu_test_support::prelude::*;

#[test]
fn test_1() -> Result {
    let code = "
    echo 1..5 | into string | str join
    ";

    test().run(code).expect_value_eq("12345")
}

#[test]
fn test_2() -> Result {
    let code = r#"
    echo [a b c d] | str join "<sep>"
    "#;

    test().run(code).expect_value_eq("a<sep>b<sep>c<sep>d")
}

#[test]
fn test_stream() -> Result {
    let code = "[a b c d] | filter {true} | str join .";
    test().run(code).expect_value_eq("a.b.c.d")
}

#[test]
fn test_stream_type() -> Result {
    let code = "[a b c d] | filter {true} | str join . | describe -n";
    test().run(code).expect_value_eq("string (stream)")
}

#[test]
fn construct_a_path() -> Result {
    let code = r#"
    echo [sample txt] | str join "."
    "#;

    test().run(code).expect_value_eq("sample.txt")
}
