use crate::repl::tests::run_test_contains;

const TEST_SCRIPT: &str = r#"
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
fn test_one_level_alias() {
    let input = format!("{TEST_SCRIPT}\n t1a");
    let expected = "This is test 1";
    run_test_contains(&input, expected).unwrap();
}

#[test]
fn test_two_level_alias() {
    let input = format!("{TEST_SCRIPT}\n t2a");
    let expected = "This is test 2";
    run_test_contains(&input, expected).unwrap();
}

#[test]
fn test_three_level_alias() {
    let input = format!("{TEST_SCRIPT}\n t3a");
    let expected = "This is test 3";
    run_test_contains(&input, expected).unwrap();
}
