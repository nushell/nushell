use nu_test_support::{nu, pipeline};

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn reduce_table_column() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | get total
<<<<<<< HEAD
        | reduce -f 20 { $it + (math eval $"($acc)^1.05")}
=======
        | reduce -f 20 { $it.item + (math eval $"($item.acc)^1.05")}
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        | into string -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn reduce_table_column_with_path() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | reduce -f 20 { $it.total + (math eval $"($acc)^1.05")}
=======
        [{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]
        | reduce -f 20 { $it.item.total + (math eval $"($item.acc)^1.05")}
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        | into string -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn reduce_rows_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        echo a,b 1,2 3,4
        | split column ,
        | headers
        | reduce -f 1.6 { $acc * ($it.a | str to-int) + ($it.b | str to-int) }
=======
        [[a,b]; [1,2] [3,4]]
        | reduce -f 1.6 { $it.acc * ($it.item.a | into int) + ($it.item.b | into int) }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
        )
    );

    assert_eq!(actual.out, "14.8");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn reduce_numbered_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo one longest three bar
<<<<<<< HEAD
        | reduce -n { if ($it.item | str length) > ($acc.item | str length) {echo $it} {echo $acc}}
=======
        | reduce -n { if ($it.item.item | str length) > ($it.acc.item | str length) {echo $it.item} else {echo $it.acc}}
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        | get index
        "#
        )
    );

    assert_eq!(actual.out, "1");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn reduce_numbered_integer_addition_example() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [1 2 3 4]
<<<<<<< HEAD
        | reduce -n { $acc.item + $it.item }
=======
        | reduce -n { $it.acc.item + $it.item.item }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        | get item
        "#
        )
    );

    assert_eq!(actual.out, "10");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn folding_with_tables() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [10 20 30 40]
        | reduce -f [] {
<<<<<<< HEAD
            with-env [value $it] {
              echo $acc | append (10 * ($nu.env.value | str to-int))
=======
            with-env [value $it.item] {
              echo $acc | append (10 * ($env.value | into int))
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            }
          }
        | math sum
        "#
        )
    );

    assert_eq!(actual.out, "1000");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn error_reduce_fold_type_mismatch() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        echo a b c | reduce -f 0 { $acc + $it }
=======
        echo a b c | reduce -f 0 { $it.acc + $it.item }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
        )
    );

<<<<<<< HEAD
    assert!(actual.err.contains("Coercion"));
}

=======
    assert!(actual.err.contains("mismatch"));
}

// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn error_reduce_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        reduce { $acc + $it }
=======
        reduce { $it.$acc + $it.item }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
        )
    );

    assert!(actual.err.contains("needs input"));
}
