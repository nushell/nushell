use nu_test_support::nu;

#[test]
fn try_succeed() {
    let output = nu!("try { 345 } catch { echo 'hello' }");

    assert!(output.out.contains("345"));
}

#[test]
fn try_catch() {
    let output = nu!("try { foobarbaz } catch { echo 'hello' }");

    assert!(output.out.contains("hello"));
}

#[test]
fn catch_can_access_error() {
    let output = nu!("try { foobarbaz } catch { |err| $err | get raw }");

    assert!(output.err.contains("External command failed"));
}

#[test]
fn catch_can_access_error_as_dollar_in() {
    let output = nu!("try { foobarbaz } catch { $in | get raw }");

    assert!(output.err.contains("External command failed"));
}

#[test]
fn external_failed_should_be_caught() {
    let output = nu!("try { nu --testbin fail; echo 'success' } catch { echo 'fail' }");

    assert!(output.out.contains("fail"));
}

#[test]
fn loop_try_break_should_be_successful() {
    let output =
        nu!("loop { try { print 'successful'; break } catch { print 'failed'; continue } }");

    assert_eq!(output.out, "successful");
}

#[test]
fn loop_catch_break_should_show_failed() {
    let output = nu!("loop {
            try { invalid 1;
            continue; } catch { print 'failed'; break }
        }
        ");

    assert_eq!(output.out, "failed");
}

#[test]
fn loop_try_ignores_continue() {
    let output = nu!("mut total = 0;
        for i in 0..10 {
            try { if ($i mod 2) == 0 {
            continue;}
            $total += 1
        } catch { echo 'failed'; break }
        }
        echo $total
        ");

    assert_eq!(output.out, "5");
}

#[test]
fn loop_try_break_on_command_should_show_successful() {
    let output = nu!("loop { try { ls; break } catch { echo 'failed';continue }}");

    assert!(!output.out.contains("failed"));
}

#[test]
fn catch_block_can_use_error_object() {
    let output = nu!("try {1 / 0} catch {|err| print ($err | get msg)}");
    assert_eq!(output.out, "Division by zero.")
}

// This test is disabled on Windows because they cause a stack overflow in CI (but not locally!).
// For reasons we don't understand, the Windows CI runners are prone to stack overflow.
// TODO: investigate so we can enable on Windows
#[cfg(not(target_os = "windows"))]
#[test]
fn can_catch_infinite_recursion() {
    let actual = nu!(r#"
            def bang [] { try { bang } catch { "Caught infinite recursion" } }; bang
        "#);
    assert_eq!(actual.out, "Caught infinite recursion");
}

#[test]
fn exit_code_available_in_catch() {
    let actual = nu!("try { nu -c 'exit 42' } catch { $env.LAST_EXIT_CODE }");
    assert_eq!(actual.out, "42");
}
