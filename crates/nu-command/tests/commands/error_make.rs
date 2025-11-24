use nu_test_support::nu;

// Required columns present
#[test]
fn error_make_empty() {
    let actual = nu!("error make {}");
    assert!(actual.err.contains("Cannot find column 'msg'"));
}

#[test]
fn error_make_no_label_text() {
    let actual = nu!("error make {msg:a,label:{span:{start:1,end:1}}}");
    assert!(
        actual
            .err
            .contains("diagnostic code: nu::shell::cant_convert")
    );
}

#[test]
fn error_label_works() {
    let actual = nu!("error make {msg:foo,label:{text:unseen}}");

    assert!(actual.err.contains(": unseen"));
}

#[test]
fn error_labels_list_works() {
    // Intentionally no space so this gets the main label bits
    let actual = nu!("error make {msg:foo,labels:[{text:unseen},{text:hidden}]}");

    assert!(actual.err.contains(": unseen"));
    assert!(actual.err.contains(": hidden"));
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

    assert!(actual.err.contains("Unable to parse Span."));
    assert!(actual.err.contains("`end` must not be less than `start`"));
}

#[test]
fn error_url_works() {
    let actual = nu!("error make {msg:bar,url:'https://example.com'}");
    assert!(
        actual
            .err
            .contains("For more details, see:\nhttps://example.com")
    );
}

#[test]
fn error_code_works() {
    let actual = nu!("error make {msg:bar,code:'foo::bar'}");
    assert!(actual.err.contains("diagnostic code: foo::bar"));
}

#[test]
fn error_check_deep() {
    let actual = nu!("error make {msg:foo,inner:[{msg:bar}]}");

    assert!(actual.err.contains("Error: bar"));
    assert!(actual.err.contains("Error: foo"));
}

#[test]
fn error_chained() {
    let actual = nu!("try {
            error make {msg:foo,inner:[{msg:bar}]}
        } catch {
            error make {msg:baz}
        }");

    assert!(actual.err.contains("Error: foo"));
    assert!(actual.err.contains("Error: baz"));
    assert!(actual.err.contains("Error: bar"));
}

#[test]
fn error_bad_label() {
    let actual = nu!("
        error make {
            msg:foo
            inner:[{msg:bar}]
            labels:foobar
        }
    ");

    assert!(!actual.err.contains("Error: foo"));
    assert!(
        actual
            .err
            .contains("diagnostic code: nu::shell::cant_convert")
    );
}

#[test]
fn check_help_line() {
    let actual = nu!("error make {msg:foo help: `Custom help line`}");

    assert!(actual.err.contains("Custom help line"));
}

#[test]
fn error_simple_chain() {
    let actual = nu!("
        try {
            error make foo
        } catch {
            error make bar
        }
    ");

    assert!(actual.err.contains("Error: foo"));
    assert!(actual.err.contains("Error: bar"));
}

#[test]
fn error_simple_string() {
    let actual = nu!("error make foo");
    assert!(actual.err.contains("Error: foo"));
}
