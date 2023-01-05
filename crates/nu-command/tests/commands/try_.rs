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

#[test]
fn loop_try_break_should_be_successful() {
    let output = nu!(
        cwd: ".",
        "loop { try { echo 'successful'; break } catch { echo 'failed'; continue } }"
    );

    assert!(output.out.contains("successful"));
}

#[test]
fn loop_catch_break_should_show_failed() {
    let output = nu!(
        cwd: ".",
        "loop {
            try { invalid 1;
            continue; } catch { echo 'failed'; break } 
        }
        "
    );

    assert!(output.out.contains("failed"));
}

#[test]
fn loop_try_break_on_command_should_show_successful() {
    let output = nu!(
        cwd: ".",
        "loop { try { ls; break } catch { echo 'failed';continue }}"
    );

    assert!(!output.out.contains("failed"));
}
