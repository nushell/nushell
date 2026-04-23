use crate::repl::tests::{TestResult, run_test_contains};

const ALIAS_SUB_COMMAND_TEST_SCRIPT: &str = r#"
    def test1 [] {
      echo "This is test 1"
    }
    def "test1 test2" [] {
      echo "This is test 2"
    }
    def "test1 test2 test3" [] {
      echo "This is test 3"
    }
    alias t1a = test1
    alias t2a = test1 test2
    alias t3a = test1 test2 test3
    "#;

#[test]
fn test_one_level_alias() -> TestResult {
    let input = format!("{ALIAS_SUB_COMMAND_TEST_SCRIPT}\n t1a");
    let expected = "This is test 1";
    run_test_contains(&input, expected)
}

#[test]
fn test_two_level_alias() -> TestResult {
    let input = format!("{ALIAS_SUB_COMMAND_TEST_SCRIPT}\n t2a");
    let expected = "This is test 2";
    run_test_contains(&input, expected)
}

#[test]
fn test_three_level_alias() -> TestResult {
    let input = format!("{ALIAS_SUB_COMMAND_TEST_SCRIPT}\n t3a");
    let expected = "This is test 3";
    run_test_contains(&input, expected)
}

#[test]
fn test_non_shadowed() -> TestResult {
    run_test_contains(
        r#"
            let x = 10
            alias xx = echo $x
            xx
        "#,
        "10",
    )
}

#[test]
fn test_shadowed() -> TestResult {
    run_test_contains(
        r#"
            let x = 10
            alias xx = echo $x
            let x = 20
            xx
        "#,
        "10",
    )
}

#[test]
fn test_mut() -> TestResult {
    run_test_contains(
        r#"
            mut x = 10
            alias xx = echo $x
            $x = 20
            xx
        "#,
        "20",
    )
}
