use nu_test_support::{playground::Playground, prelude::*};
use std::fs;

#[test]
fn tee_save_values_to_file() -> Result {
    Playground::setup("tee_save_values_to_file_test", |dirs, _sandbox| {
        test()
            .cwd(dirs.test())
            .run("1..5 | tee { save copy.txt }")
            .expect_value_eq([1, 2, 3, 4, 5])?;
        assert_eq!(
            "1\n2\n3\n4\n5\n",
            fs::read_to_string(dirs.test().join("copy.txt"))?
        );

        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV)]
fn tee_save_stdout_to_file() -> Result {
    Playground::setup("tee_save_stdout_to_file_test", |dirs, _sandbox| {
        let code = r#"
            $env.FOO = "teststring"
            echo_env FOO | tee { save copy.txt }
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("teststring")?;

        assert_eq!(
            "teststring\n",
            fs::read_to_string(dirs.test().join("copy.txt"))?
        );

        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn tee_save_stderr_to_file() -> Result {
    Playground::setup("tee_save_stderr_to_file_test", |dirs, _sandbox| {
        let code = r#"
            $env.FOO = "teststring"
            do { echo_env_stderr FOO }
            | tee --stderr { save copy.txt }
            | complete
            | get stderr
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("teststring\n")?;

        assert_eq!(
            "teststring\n",
            fs::read_to_string(dirs.test().join("copy.txt"))?
        );

        Ok(())
    })
}

#[test]
#[deps(NU)]
fn tee_single_value_streamable() -> Result {
    let code = "'Hello, world!' | tee { print -e } | print";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;

    assert_eq!(result.exit_code, 0);
    assert_eq!("Hello, world!", result.stdout);
    // FIXME: note the lack of newline: this is a consequence of converting the string to a stream
    // for now, but most likely the printer should be checking whether a string stream ends with a
    // newline and adding it unless no_newline is true
    assert_eq!("Hello, world!", result.stderr);

    Ok(())
}

#[test]
#[deps(NU)]
fn tee_single_value_non_streamable() -> Result {
    // Non-streamable values don't have any synchronization point, so we have to wait.
    let code = "
        500 | tee { print -e } | print
        sleep 0.1sec
    ";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;

    assert_eq!(result.exit_code, 0);
    assert_eq!("500\n", result.stdout);
    assert_eq!("500\n", result.stderr);

    Ok(())
}
