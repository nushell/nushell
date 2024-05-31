use nu_test_support::{nu, pipeline};

#[test]
fn reduce_table_column() {
    let actual = nu!(pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | get total
        | reduce --fold 20 { |it, acc| $it + $acc ** 1.05}
        | into string -d 1
        "#
    ));

    assert_eq!(actual.out, "180.6");
}

#[test]
fn reduce_table_column_with_path() {
    let actual = nu!(pipeline(
        "
        [{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]
        | reduce --fold 20 { |it, acc| $it.total + $acc ** 1.05}
        | into string -d 1
        "
    ));

    assert_eq!(actual.out, "180.6");
}

#[test]
fn reduce_rows_example() {
    let actual = nu!(pipeline(
        "
        [[a,b]; [1,2] [3,4]]
        | reduce --fold 1.6 { |it, acc| $acc * ($it.a | into int) + ($it.b | into int) }
        "
    ));

    assert_eq!(actual.out, "14.8");
}

#[test]
fn reduce_with_return_in_closure() {
    let actual = nu!(pipeline(
        "
        [1, 2] | reduce --fold null { |it, state|
            if $it == 1 {
                return 10
            };
            return ($it * $state)
        }
        "
    ));

    assert_eq!(actual.out, "20");
    assert!(actual.err.is_empty());
}

#[test]
fn reduce_enumerate_example() {
    let actual = nu!(pipeline(
        "
        echo one longest three bar | enumerate
        | reduce { |it, acc| if ($it.item | str length) > ($acc.item | str length) {echo $it} else {echo $acc}}
        | get index
        "
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn reduce_enumerate_integer_addition_example() {
    let actual = nu!(pipeline(
        "
        echo [1 2 3 4]
        | enumerate
        | reduce { |it, acc| { index: ($it.index) item: ($acc.item + $it.item)} }
        | get item
        "
    ));

    assert_eq!(actual.out, "10");
}

#[test]
fn folding_with_tables() {
    let actual = nu!(pipeline(
        "
        echo [10 20 30 40]
        | reduce --fold [] { |it, acc|
            with-env { value: $it } {
              echo $acc | append (10 * ($env.value | into int))
            }
          }
        | math sum
        "
    ));

    assert_eq!(actual.out, "1000");
}

#[test]
fn error_reduce_fold_type_mismatch() {
    let actual = nu!(pipeline(
        "echo a b c | reduce --fold 0 { |it, acc| $acc + $it }"
    ));

    assert!(actual.err.contains("mismatch"));
}

#[test]
fn error_reduce_empty() {
    let actual = nu!(pipeline("reduce { |it, acc| $acc + $it }"));

    assert!(actual.err.contains("needs input"));
}

#[test]
fn enumerate_reduce_example() {
    let actual = nu!(pipeline(
        "[one longest three bar] | enumerate | reduce {|it, acc| if ($it.item | str length) > ($acc.item | str length) { $it } else { $acc }} | get index"
    ));

    assert_eq!(actual.out, "1");
}
