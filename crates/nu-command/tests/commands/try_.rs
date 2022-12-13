use nu_test_support::nu;

#[test]
fn try_succeed() {
    let output = nu!(
        cwd: ".",
        "try { 345 } catch { echo 'hello' }"
    );

    assert!(output.out.contains("345"));
}

#[test]
fn try_catch() {
    let output = nu!(
        cwd: ".",
        "try { foobarbaz } catch { echo 'hello' }"
    );

    assert!(output.out.contains("hello"));
}

#[test]
fn catch_can_access_error() {
    let output = nu!(
        cwd: ".",
        "try { foobarbaz } catch { |err| $err }"
    );

    assert!(output.err.contains("External command failed"));
}

#[test]
fn catch_can_access_error_as_dollar_in() {
    let output = nu!(
        cwd: ".",
        "try { foobarbaz } catch { $in }"
    );

    assert!(output.err.contains("External command failed"));
}

#[test]
fn external_failed_should_be_catched() {
    let output = nu!(
        cwd: ".",
        "try { nu --testbin fail; echo 'success' } catch { echo 'fail' }"
    );

    assert!(output.out.contains("fail"));
}
