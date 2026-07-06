use nu_test_support::prelude::*;

#[test]
fn reduce_table_column() -> Result {
    let code = r#"
        [[month, total]; [2, 30], [3, 10], [4, 3], [5, 60]]
        | get total
        | reduce --fold 20 { |it, acc| $it + $acc * 2 }
    "#;
    test().run(code).expect_value_eq(666)
}

#[test]
fn reduce_table_column_with_path() -> Result {
    let code = "
        [[month, total]; [2, 30], [3, 10], [4, 3], [5, 60]]
        | reduce --fold 20 { |it, acc| $it.total + $acc * 2 }
    ";
    test().run(code).expect_value_eq(666)
}

#[test]
fn reduce_rows_example() -> Result {
    let code = "
        [[a,b]; [1,2] [3,4]]
        | reduce --fold 1.6 {|it, acc|
            $acc * ($it.a | into int) + ($it.b | into int)
        }
    ";
    test().run(code).expect_value_eq(14.8)
}

#[test]
fn reduce_with_return_in_closure() -> Result {
    let code = "
        [1, 2] | reduce --fold null { |it, state|
            if $it == 1 {
                return 10
            };
            return ($it * $state)
        }
    ";
    test().run(code).expect_value_eq(20)
}

#[test]
fn reduce_enumerate_example() -> Result {
    let code = "
        [one longest three bar]
        | enumerate
        | reduce {|it, acc|
            if ($it.item | str length) > ($acc.item | str length) {
                $it
            } else {
                $acc
            }
        }
        | get index
    ";
    test().run(code).expect_value_eq(1)
}

#[test]
fn reduce_enumerate_integer_addition_example() -> Result {
    let code = "
        [1 2 3 4]
        | enumerate
        | reduce {|it, acc|
            {
                index: ($it.index)
                item: ($acc.item + $it.item)
            }
        }
        | get item
    ";
    test().run(code).expect_value_eq(10)
}

#[test]
fn folding_with_tables() -> Result {
    let code = "
        [10 20 30 40]
        | reduce --fold [] { |it, acc|
            with-env { value: $it } {
              echo $acc | append (10 * ($env.value | into int))
            }
          }
        | math sum
    ";
    test().run(code).expect_value_eq(1000)
}

#[test]
fn error_reduce_fold_type_mismatch() -> Result {
    test()
        .run("echo a b c | reduce --fold 0 { |it, acc| $acc + $it }")
        .expect_error_code_eq("nu::shell::operator_incompatible_types")
}

#[test]
fn error_reduce_empty() -> Result {
    test()
        .run("reduce { |it, acc| $acc + $it }")
        .expect_error_code_eq("nu::shell::pipeline_mismatch")
}

#[test]
fn enumerate_reduce_example() -> Result {
    let code = "
        [one longest three bar]
        | enumerate
        | reduce {|it, acc|
            if ($it.item | str length) > ($acc.item | str length) {
                $it
            } else {
                $acc
            }
        }
        | get index
    ";
    test().run(code).expect_value_eq(1)
}
