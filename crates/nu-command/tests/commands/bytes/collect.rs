use nu_test_support::{nu, pipeline};

#[test]
fn test_stream() {
    let actual = nu!(pipeline(
        "
            [0x[01] 0x[02] 0x[03] 0x[04]]
            | filter {true}
            | bytes collect 0x[aa aa]
            | encode hex
        "
    ));
    assert_eq!(actual.out, "01AAAA02AAAA03AAAA04");
}

#[test]
fn test_stream_type() {
    let actual = nu!(pipeline(
        "
            [0x[01] 0x[02] 0x[03] 0x[04]]
            | filter {true}
            | bytes collect 0x[00]
            | describe -n
        "
    ));
    assert_eq!(actual.out, "binary (stream)");
}
