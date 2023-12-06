use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn limit_nice() {
    Playground::setup("limit_nice", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "ulimit -e 10"
        );

        assert!(actual.err.contains("EPERM: Operation not permitted"));
    });
}

#[test]
fn limit_set_and_get() {
    Playground::setup("limit_core_size", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "ulimit -s 100"
        );

        assert!(actual.out.is_empty());

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ulimit -s 100;
                ulimit -s
                | values
            ")
        );

        assert!(actual.out.contains("100"));
    });
}

#[test]
fn invalid_limit() {
    Playground::setup("limit_core_size", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "ulimit -c abcd"
        );

        assert!(actual.err.contains("Can't convert to rlim_t."));
    });
}
