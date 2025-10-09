use nu_test_support::nu;

#[test]
fn error_label_works() {
    let actual = nu!("error make {msg:foo label:{text:unseen}}");

    assert!(
        actual
            .err
            .contains("label at line 1, columns 1 to 10: unseen")
    );
}

#[test]
fn no_span_if_unspanned() {
    let actual = nu!("error make -u {msg:foo label:{text:unseen}}");

    assert!(!actual.err.contains("unseen"));
}

#[test]
fn error_start_bigger_than_end_should_fail() {
    let actual = nu!("
        error make {
            msg: foo
            label: {
                text: bar
                span: {start: 456 end: 123}
            }
        }
    ");

    assert!(actual.err.contains("invalid error format"));
    assert!(
        actual
            .err
            .contains("`$.label.start` should be smaller than `$.label.end`")
    );
}

#[test]
fn check_help_line() {
    let actual = nu!("error make {msg:foo help: `Custom help line`}");

    assert!(actual.err.contains("Custom help line"));
}
