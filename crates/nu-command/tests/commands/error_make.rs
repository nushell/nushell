use nu_protocol::Span;
use nu_test_support::prelude::*;

// Required columns present
#[test]
fn error_make_empty() -> Result {
    let err = test().run("error make {}").expect_shell_error()?;
    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "msg");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn error_make_no_label_text() -> Result {
    let code = "
        error make {
            msg: no_label_text,
            label: {
                span: {
                    start: 1,
                    end: 1
                }
            }
        }
    ";

    let err = test().run(code).expect_labeled_error()?;

    assert_eq!(err.msg, "no_label_text");
    assert_eq!(err.labels[0].span, Span::new(1, 1));
    Ok(())
}

#[test]
fn error_label_works() -> Result {
    let code = "
        error make {
            msg: foo,
            label: { text: unseen }
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "foo");
    assert_eq!(err.labels[0].text, "unseen");
    Ok(())
}

#[test]
fn error_labels_list_works() -> Result {
    let code = "
        error make {
            msg: foo,
            labels: [
                { text: unseen },
                { text: hidden },
            ]
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "foo");
    assert_eq!(err.labels[0].text, "unseen");
    assert_eq!(err.labels[1].text, "hidden");
    Ok(())
}

#[test]
fn no_span_if_unspanned() -> Result {
    let code = "
        error make -u {
            msg: foo
            label: { text: unseen }
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "foo");
    assert!(err.labels.is_empty());
    Ok(())
}

#[test]
fn error_start_bigger_than_end_should_fail() -> Result {
    let code = "
        error make {
            msg: foo
            label: {
                text: bar
                span: { start: 456 end: 123 }
            }
        }
    ";

    let err = test().run(code).expect_shell_error()?;
    assert_eq!(err.clone().generic_error()?, "Unable to parse Span.");
    assert_eq!(err.generic_msg()?, "`end` must not be less than `start`");
    Ok(())
}

#[test]
fn error_url_works() -> Result {
    let code = "
        error make {
            msg: bar,
            url: 'https://example.com'
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.url.unwrap(), "https://example.com");
    Ok(())
}

#[test]
fn error_code_works() -> Result {
    let code = "
        error make {
            msg: bar,
            code: 'foo::bar'
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.code.unwrap(), "foo::bar");
    Ok(())
}

#[test]
fn error_check_deep() -> Result {
    let code = "
        error make {
            msg: foo,
            inner: [{
                msg: bar
            }]
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "foo");

    let inner = err.inner[0].clone().into_labeled()?;
    assert_eq!(inner.msg, "bar");

    Ok(())
}

#[test]
fn error_chained() -> Result {
    let code = "
        try {
            error make {
                msg:foo,
                inner: [{ msg:bar }]
            }
        } catch {
            error make {
                msg:baz
            }
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    // baz -> foo -> bar
    assert_eq!(err.msg, "baz");

    let inner = err.inner[0].clone().into_labeled()?;
    assert_eq!(inner.msg, "foo");

    let deep_inner = inner.inner[0].clone().into_labeled()?;
    assert_eq!(deep_inner.msg, "bar");

    Ok(())
}

#[test]
fn error_bad_label() -> Result {
    let code = "
        error make {
            msg: foo
            inner: [{ msg: bar }]
            labels: foobar
        }
    ";

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn check_help_line() -> Result {
    let code = "
        error make {
            msg: foo,
            help: `Custom help line`
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.help.unwrap(), "Custom help line");
    Ok(())
}

#[test]
fn error_simple_chain() -> Result {
    let code = "
        try {
            error make foo
        } catch {
            error make bar
        }
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "bar");

    let inner = err.inner[0].clone().into_labeled()?;
    assert_eq!(inner.msg, "foo");

    Ok(())
}

#[test]
fn error_simple_string() -> Result {
    let err = test().run("error make foo").expect_labeled_error()?;
    assert_eq!(err.msg, "foo");
    Ok(())
}

#[test]
fn error_source() -> Result {
    let code = "
        error make {
            msg: foo
            src: { text: 'foo bar' }
            labels: [{
                text: bar 
                span: { start: 0 end: 3 } 
            }]
        }
    ";

    let err = test().run(code).expect_shell_error()?;
    let ShellError::OutsideSourceNoUrl { msg, labels, .. } = err else {
        return Err(err.into());
    };

    assert_eq!(msg, "foo");
    assert_eq!(labels[0].label().unwrap(), "bar");

    Ok(())
}
