use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn try_succeed() {
    Playground::setup("try_succeed_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "try { 345 } catch { echo 'hello' }"
        );

        assert!(output.out.contains("345"));
    })
}

#[test]
fn try_catch() {
    Playground::setup("try_catch_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "try { foobarbaz } catch { echo 'hello' }"
        );

        assert!(output.out.contains("hello"));
    })
}

#[test]
fn catch_can_access_error() {
    Playground::setup("try_catch_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "try { foobarbaz } catch { |err| $err }"
        );

        assert!(output.err.contains("External command failed"));
    })
}

#[test]
fn external_failed_should_be_catched() {
    Playground::setup("try_catch_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "try { nu --testbin fail; echo 'success' } catch { echo 'fail' }"
        );

        assert!(output.out.contains("fail"));
    })
}
