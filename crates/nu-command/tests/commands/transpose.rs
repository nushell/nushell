use nu_test_support::nu;

#[test]
fn row() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r | debug");

    assert!(actual.out.contains("foo: 1"));
}

#[test]
fn row_but_last() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r -l | debug");

    assert!(actual.out.contains("foo: 2"));
}

#[test]
fn row_but_all() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r -a | debug");

    assert!(actual.out.contains("foo: [1, 2]"));
}

#[test]
fn throw_inner_error() {
    let error_msg = "This message should show up";
    let error = format!("(error make {{ msg: \"{error_msg}\" }})");
    let actual = nu!(format!(
        "[[key value]; [foo 1] [foo 2] [{} 3]] | transpose",
        error
    ));

    assert!(actual.err.contains(error.as_str()));
}

#[test]
fn rejects_non_table_stream_input() {
    let actual = nu!("[1 2 3] | each { |it| ($it * 2) } | transpose | to nuon");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("only table"));
}
