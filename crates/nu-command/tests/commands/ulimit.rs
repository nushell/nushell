use nu_test_support::prelude::*;

// Keep resource-limit mutations out of the shared in-process test runner.
fn run_ulimit(code: &str) -> Result<CompleteResult> {
    test()
        .env("NU_TEST_ULIMIT_CODE", code)
        .run("nu -n -c $env.NU_TEST_ULIMIT_CODE | complete")
}

#[test]
#[deps(NU)]
fn limit_set_soft1() -> Result {
    let actual = run_ulimit(
        "
        let soft = (ulimit -s | first | get soft);
        ulimit -s -H $soft;
        let hard = (ulimit -s | first | get hard);
        $soft == $hard
    ",
    )?;

    assert!(actual.stdout.contains("true"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_soft2() -> Result {
    let actual = run_ulimit(
        "
        let soft = (ulimit -s | first | get soft);
        ulimit -s -H soft;
        let hard = (ulimit -s | first | get hard);
        $soft == $hard
    ",
    )?;

    assert!(actual.stdout.contains("true"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_hard1() -> Result {
    let actual = run_ulimit(
        "
        let hard = (ulimit -s | first | get hard);
        ulimit -s $hard;
        let soft = (ulimit -s | first | get soft);
        $soft == $hard
    ",
    )?;

    assert!(actual.stdout.contains("true"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_hard2() -> Result {
    let actual = run_ulimit(
        "
        let hard = (ulimit -s | first | get hard);
        ulimit -s hard;
        let soft = (ulimit -s | first | get soft);
        $soft == $hard
    ",
    )?;

    assert!(actual.stdout.contains("true"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_invalid1() -> Result {
    let actual = run_ulimit(
        "
        let hard = (ulimit -s | first | get hard);
        match $hard {
            \"unlimited\" => { echo \"unlimited\" },
            $x => {
                let new = $x + 1;
                ulimit -s $new
            }
        }
    ",
    )?;

    assert!(
        actual.stdout.contains("unlimited")
            || actual.stderr.contains("EPERM: Operation not permitted")
    );
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
#[test]
#[deps(NU)]
fn limit_set_invalid2() -> Result {
    let actual = run_ulimit(
        "
        let val = -100;
        ulimit -c $val
    ",
    )?;

    assert!(actual.stderr.contains("can't convert i64 to rlim_t"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_invalid3() -> Result {
    let actual = run_ulimit(
        "
        ulimit -c abcd
    ",
    )?;

    assert!(
        actual
            .stderr
            .contains("Only unlimited, soft and hard are supported for strings")
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_invalid4() -> Result {
    let actual = run_ulimit(
        "
        ulimit -c 100.0
    ",
    )?;

    assert!(actual.stderr.contains("string, int or filesize required"));
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_invalid5() -> Result {
    use nix::sys::resource::rlim_t;

    let max = (rlim_t::MAX / 1024) + 1;

    let actual = run_ulimit(&format!(
        r#"
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
        "#
    ))?;

    assert_eq!(actual.stdout.trim(), "unlimited");
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_filesize1() -> Result {
    let actual = run_ulimit(
        "
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
    ",
    )?;

    let stdout = actual.stdout.trim();
    assert!(stdout == "1024" || stdout == "hard limit too small");
    Ok(())
}

#[test]
#[deps(NU)]
fn limit_set_filesize2() -> Result {
    let actual = run_ulimit(
        "
        ulimit -n 10Kib
    ",
    )?;

    assert!(
        actual
            .stderr
            .contains("filesize is not compatible with resource RLIMIT_NOFILE")
    );
    Ok(())
}
