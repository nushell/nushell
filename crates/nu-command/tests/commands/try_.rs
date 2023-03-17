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
        "try { foobarbaz } catch { |err| $err | get raw }"
    );

    assert!(output.err.contains("External command failed"));
}

#[test]
fn catch_can_access_error_as_dollar_in() {
    let output = nu!(
        cwd: ".",
        "try { foobarbaz } catch { $in | get raw }"
    );

    assert!(output.err.contains("External command failed"));
}

#[test]
fn external_failed_should_be_caught() {
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
        "loop { try { print 'successful'; break } catch { print 'failed'; continue } }"
    );

    assert_eq!(output.out, "successful");
}

#[test]
fn loop_catch_break_should_show_failed() {
    let output = nu!(
        cwd: ".",
        "loop {
            try { invalid 1;
            continue; } catch { print 'failed'; break }
        }
        "
    );

    assert_eq!(output.out, "failed");
}

#[test]
fn loop_try_ignores_continue() {
    let output = nu!(
        cwd: ".",
        "mut total = 0;
        for i in 0..10 {
            try { if ($i mod 2) == 0 {
            continue;}
            $total += 1
        } catch { echo 'failed'; break }
        }
        echo $total
        "
    );

    assert_eq!(output.out, "5");
}

#[test]
fn loop_try_break_on_command_should_show_successful() {
    let output = nu!(
        cwd: ".",
        "loop { try { ls; break } catch { echo 'failed';continue }}"
    );

    assert!(!output.out.contains("failed"));
}

#[test]
fn catch_block_can_use_error_object() {
    let output = nu!(
        cwd: ".",
        "try {1 / 0} catch {|err| print ($err | get msg)}"
    );
    assert_eq!(output.out, "Division by zero.")
}
