use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(not(windows))]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt err> a; cat a"
        );

        assert!(output.err.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "type asdfasdfasdf.txt err> a; type a"
        );

        assert!(output.err.contains("asdfasdfasdf.txt"));
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

        assert!(output.err.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "type asdfasdfasdf.txt out+err> a; type a"
        );

        assert!(output.err.contains("asdfasdfasdf.txt"));
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
