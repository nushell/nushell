use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn limit_set_soft1() {
    Playground::setup("limit_set_soft1", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let soft = (ulimit -s | first | get soft);
            ulimit -s -H $soft;
            let hard = (ulimit -s | first | get hard);
            $soft == $hard
        ");

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_soft2() {
    Playground::setup("limit_set_soft2", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let soft = (ulimit -s | first | get soft);
            ulimit -s -H soft;
            let hard = (ulimit -s | first | get hard);
            $soft == $hard
        ");

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_hard1() {
    Playground::setup("limit_set_hard1", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let hard = (ulimit -s | first | get hard);
            ulimit -s $hard;
            let soft = (ulimit -s | first | get soft);
            $soft == $hard
                   ");

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_hard2() {
    Playground::setup("limit_set_hard2", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let hard = (ulimit -s | first | get hard);
            ulimit -s hard;
            let soft = (ulimit -s | first | get soft);
            $soft == $hard
        ");

        assert!(actual.out.contains("true"));
    });
}

#[test]
fn limit_set_invalid1() {
    Playground::setup("limit_set_invalid1", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let hard = (ulimit -s | first | get hard);
            match $hard {
                \"unlimited\" => { echo \"unlimited\" },
                $x => {
                    let new = $x + 1;
                    ulimit -s $new
                }
            }
        ");

        assert!(
            actual.out.contains("unlimited")
                || actual.err.contains("EPERM: Operation not permitted")
        );
    });
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
#[test]
fn limit_set_invalid2() {
    Playground::setup("limit_set_invalid2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                let val = -100;
                ulimit -c $val
            "
        );

        assert!(actual.err.contains("can't convert i64 to rlim_t"));
    });
}

#[test]
fn limit_set_invalid3() {
    Playground::setup("limit_set_invalid3", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                ulimit -c abcd
            "
        );

        assert!(
            actual
                .err
                .contains("Only unlimited, soft and hard are supported for strings")
        );
    });
}

#[test]
fn limit_set_invalid4() {
    Playground::setup("limit_set_invalid4", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                ulimit -c 100.0
            "
        );

        assert!(actual.err.contains("string, int or filesize required"));
    });
}

#[test]
fn limit_set_invalid5() {
    use nix::sys::resource::rlim_t;

    let max = (rlim_t::MAX / 1024) + 1;

    Playground::setup("limit_set_invalid5", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), format!(r#"
            let hard = (ulimit -c | first | get hard)
            match $hard {{
                "unlimited" => {{
                    ulimit -c -S 0
                    ulimit -c {max}
                    ulimit -c
                    | first
                    | get soft
                }},
                _ => {{
                    echo "unlimited"
                }}
            }}
        "#));

        assert!(actual.out.eq("unlimited"));
    });
}

#[test]
fn limit_set_filesize1() {
    Playground::setup("limit_set_filesize1", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            let hard = (ulimit -c | first | get hard);
            match $hard {
                \"unlimited\" => {
                    ulimit -c 1Mib;
                    ulimit -c
                    | first
                    | get soft
                },
                $x if $x >= 1024 * 1024 => {
                    ulimit -c 1Mib;
                    ulimit -c
                    | first
                    | get soft
                }
                _ => {
                    echo \"hard limit too small\"
                }
            }
        ");

        assert!(actual.out.eq("1024") || actual.out.eq("hard limit too small"));
    });
}

#[test]
fn limit_set_filesize2() {
    Playground::setup("limit_set_filesize2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                ulimit -n 10Kib
            "
        );

        assert!(
            actual
                .err
                .contains("filesize is not compatible with resource RLIMIT_NOFILE")
        );
    });
}
