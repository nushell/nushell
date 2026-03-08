use std::fmt::Display;

use nu_test_support::prelude::*;

fn echo_stdout(msg: impl Display) -> impl Display {
    #[cfg(windows)]
    return format!(r#"cmd.exe /c "echo {msg}""#);

    #[cfg(not(windows))]
    return format!(r#"sh "-c" "echo {msg}""#);
}

fn echo_env_stdout(key: impl Display) -> impl Display {
    #[cfg(windows)]
    return format!(r#"cmd.exe /c "echo %{key}%""#);

    #[cfg(not(windows))]
    return format!(r#"sh "-c" "echo ${key}""#);
}

fn echo_env_stderr(key: impl Display) -> impl Display {
    #[cfg(windows)]
    return format!(r#"cmd.exe /c "echo %{key}% 1>&2""#);

    #[cfg(not(windows))]
    return format!(r#"sh "-c" "echo ${key} 1>&2""#);
}

#[test]
fn timeit_show_stdout() -> Result {
    let code = format!(
        r#"
        timeit --output {{ do {{ run-external {} }} | complete }}
        | get output.stdout
        | str trim
    "#,
        echo_stdout("abcdefg")
    );
    let outcome: String = test().inherit_path().run(code)?;
    assert_eq!(outcome, "abcdefg");
    Ok(())
}

#[test]
fn timeit_show_stderr() -> Result {
    let stdout_code = format!(
        r#"
        with-env {{FOO: bar, FOO2: baz}} {{
            timeit --output {{ do {{ run-external {} }} | complete }}
            | get output.stdout
        }}
    "#,
        echo_env_stdout("FOO")
    );
    let stdout: String = test().inherit_path().run(stdout_code)?;
    assert_contains("bar", stdout);

    let stderr_code = format!(
        r#"
        with-env {{FOO: bar, FOO2: baz}} {{
            timeit --output {{ do {{ run-external {} }} | complete }}
            | get output.stderr
        }}
    "#,
        echo_env_stderr("FOO2")
    );
    let stderr: String = test().inherit_path().run(stderr_code)?;
    assert_contains("baz", stderr);
    Ok(())
}

#[test]
fn timeit_show_output() -> Result {
    let code = "timeit --output { 'this is a test' } | get output";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "this is a test");
    Ok(())
}
