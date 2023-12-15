use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn limit_set_soft1() {
    Playground::setup("limit_set_soft1", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                let soft = (ulimit -s | first | get soft | into string);
                ulimit -s -H $soft;
                let hard = (ulimit -s | first | get hard | into string);
                $soft == $hard
            "
        ));

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_soft2() {
    Playground::setup("limit_set_soft2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                let soft = (ulimit -s | first | get soft | into string);
                ulimit -s -H soft;
                let hard = (ulimit -s | first | get hard | into string);
                $soft == $hard
            "
        ));

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_hard1() {
    Playground::setup("limit_set_hard1", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                let hard = (ulimit -s | first | get hard | into string);
                ulimit -s $hard;
                let soft = (ulimit -s | first | get soft | into string);
                $soft == $hard
           "
        ));

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_hard2() {
    Playground::setup("limit_set_hard2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                let hard = (ulimit -s | first | get hard | into string);
                ulimit -s hard;
                let soft = (ulimit -s | first | get soft | into string);
                $soft == $hard
            "
        ));

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_invalid1() {
    Playground::setup("limit_set_invalid1", |dirs, _sandbox| {
        let actual = nu!(
        cwd: dirs.test(), pipeline(
        "
            let hard = (ulimit -s | first | get hard);
            match $hard {
                \"unlimited\" => { echo \"unlimited\" },
                $x => {
                    let new = ($x + 1 | into string);
                    ulimit -s $new
                }
            }
        "
        ));

        assert!(
            actual.out.contains("unlimited")
                || actual.err.contains("EPERM: Operation not permitted")
        );
    });
}

#[test]
fn limit_set_invalid2() {
    Playground::setup("limit_set_invalid2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                ulimit -c abcd
            "
        );

        assert!(actual.err.contains("Can't convert to rlim_t."));
    });
}
