use nu_test_support::{nu, pipeline};

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reduce_table_column() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | get total
        | reduce -f 20 { $it.item + (math eval $"($item.acc)^1.05")}
        | into string -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reduce_table_column_with_path() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        [{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]
        | reduce -f 20 { $it.item.total + (math eval $"($item.acc)^1.05")}
        | into string -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reduce_rows_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        [[a,b]; [1,2] [3,4]]
        | reduce -f 1.6 { $it.acc * ($it.item.a | into int) + ($it.item.b | into int) }
        "#
        )
    );

    assert_eq!(actual.out, "14.8");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reduce_numbered_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo one longest three bar
        | reduce -n { if ($it.item.item | str length) > ($it.acc.item | str length) {echo $it.item} else {echo $it.acc}}
        | get index
        "#
        )
    );

    assert_eq!(actual.out, "1");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn reduce_numbered_integer_addition_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [1 2 3 4]
        | reduce -n { $it.acc.item + $it.item.item }
        | get item
        "#
        )
    );

    assert_eq!(actual.out, "10");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn folding_with_tables() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [10 20 30 40]
        | reduce -f [] {
            with-env [value $it.item] {
              echo $acc | append (10 * ($env.value | into int))
            }
          }
        | math sum
        "#
        )
    );

    assert_eq!(actual.out, "1000");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn error_reduce_fold_type_mismatch() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo a b c | reduce -f 0 { $it.acc + $it.item }
        "#
        )
    );

    assert!(actual.err.contains("mismatch"));
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn error_reduce_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        reduce { $it.$acc + $it.item }
        "#
        )
    );

    assert!(actual.err.contains("needs input"));
}
