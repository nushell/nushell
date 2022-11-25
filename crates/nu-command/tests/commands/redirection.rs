use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(not(windows))]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt err> a.txt; cat a.txt"
        );

        assert!(output.out.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "dir missingapplication err> a; (open a | size).bytes >= 16"
        );

        assert!(output.out.contains("true"));
    })
}

#[cfg(not(windows))]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt out+err> a; cat a"
        );

        assert!(output.out.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "dir missingapplication out+err> a; (open a | size).bytes >= 16"
        );

        assert!(output.out.contains("true"));
    })
}

#[test]
fn redirect_out() {
    Playground::setup("redirect_out_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "echo 'hello' out> a; open a"
        );

        assert!(output.out.contains("hello"));
    })
}
