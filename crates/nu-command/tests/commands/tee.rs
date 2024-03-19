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
