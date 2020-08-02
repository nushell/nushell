use nu_test_support::{nu, pipeline};

#[test]
fn reduce_table_column() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | get total
        | reduce -f 20 { = $it + $( math eval `{{$acc}}^1.05` )}
        | str from -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | reduce -f 20 { = $it.total + $( math eval `{{$acc}}^1.05` )}
        | str from -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

#[test]
fn error_reduce_fold_type_mismatch() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo a b c | reduce -f 0 { = $acc + $it }
        "#
        )
    );

    assert!(actual.err.contains("Coercion"));
}

#[test]
fn error_reduce_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        reduce { = $acc + $it }
        "#
        )
    );

    assert!(actual.err.contains("empty input"));
}
