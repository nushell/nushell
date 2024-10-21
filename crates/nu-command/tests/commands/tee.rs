use nu_test_support::{fs::file_contents, nu, playground::Playground};

#[test]
fn tee_save_values_to_file() {
    Playground::setup("tee_save_values_to_file_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            r#"1..5 | tee { save copy.txt } | to text"#
        );
        assert_eq!("12345", output.out);
        assert_eq!(
            "1\n2\n3\n4\n5\n",
            file_contents(dirs.test().join("copy.txt"))
        );
    })
}

#[test]
fn tee_save_stdout_to_file() {
    Playground::setup("tee_save_stdout_to_file_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            r#"
                $env.FOO = "teststring"
                nu --testbin echo_env FOO | tee { save copy.txt }
            "#
        );
        assert_eq!("teststring", output.out);
        assert_eq!("teststring\n", file_contents(dirs.test().join("copy.txt")));
    })
}

#[test]
fn tee_save_stderr_to_file() {
    Playground::setup("tee_save_stderr_to_file_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "\
                $env.FOO = \"teststring\"; \
                do { nu --testbin echo_env_stderr FOO } | \
                    tee --stderr { save copy.txt } | \
                    complete | \
                    get stderr
            "
        );
        assert_eq!("teststring", output.out);
        assert_eq!("teststring\n", file_contents(dirs.test().join("copy.txt")));
    })
}

#[test]
fn tee_single_value_streamable() {
    let actual = nu!("'Hello, world!' | tee { print -e } | print");
    assert!(actual.status.success());
    assert_eq!("Hello, world!", actual.out);
    // FIXME: note the lack of newline: this is a consequence of converting the string to a stream
    // for now, but most likely the printer should be checking whether a string stream ends with a
    // newline and adding it unless no_newline is true
    assert_eq!("Hello, world!", actual.err);
}

#[test]
fn tee_single_value_non_streamable() {
    // Non-streamable values don't have any synchronization point, so we have to wait.
    let actual = nu!("500 | tee { print -e } | print; sleep 1sec");
    assert!(actual.status.success());
    assert_eq!("500", actual.out);
    assert_eq!("500\n", actual.err);
}
