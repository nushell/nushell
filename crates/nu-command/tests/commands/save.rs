use nu_test_support::fs::file_contents;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::io::Write;

#[test]
fn writes_out_csv() {
    Playground::setup("save_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("cargo_sample.csv");

        nu!(
            cwd: dirs.root(),
            r#"[[name, version, description, license, edition]; [nu, "0.14", "A new type of shell", "MIT", "2018"]] | save save_test_2/cargo_sample.csv"#,
        );

        let actual = file_contents(expected_file);
        println!("{}", actual);
        assert!(actual.contains("nu,0.14,A new type of shell,MIT,2018"));
    })
}

#[test]
fn writes_out_list() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("list_sample.txt");

        nu!(
            cwd: dirs.root(),
            r#"[a b c d] | save save_test_3/list_sample.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "a\nb\nc\nd\n")
    })
}

#[test]
fn save_append_will_create_file_if_not_exists() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("new-file.txt");

        nu!(
            cwd: dirs.root(),
            r#"'hello' | save --raw --append save_test_3/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{}", actual);
        assert_eq!(actual, "hello");
    })
}

#[test]
fn save_append_will_not_overwrite_content() {
    Playground::setup("save_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("new-file.txt");

        {
            let mut file =
                std::fs::File::create(&expected_file).expect("Failed to create test file");
            file.write_all("hello ".as_bytes())
                .expect("Failed to write to test file");
            file.flush().expect("Failed to flush io")
        }

        nu!(
            cwd: dirs.root(),
            r#"'world' | save --append save_test_4/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{}", actual);
        assert_eq!(actual, "hello world");
    })
}

#[test]
fn save_stderr_and_stdout_to_same_file() {
    Playground::setup("save_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("new-file.txt");

        nu!(
            cwd: dirs.root(),
            r#"
            let-env FOO = "bar";
            let-env BAZ = "ZZZ";
            do -i {nu -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -r save_test_5/new-file.txt --stderr save_test_5/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{}, {}", actual, actual.contains("ZZZ"));
        assert!(actual.contains("bar"));
        assert!(actual.contains("ZZZ"));
    })
}

#[test]
fn save_stderr_and_stdout_to_diff_file() {
    Playground::setup("save_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        nu!(
            cwd: dirs.root(),
            r#"
            let-env FOO = "bar";
            let-env BAZ = "ZZZ";
            do -i {nu -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -r save_test_6/log.txt --stderr save_test_6/err.txt"#,
        );

        let actual = file_contents(expected_file);
        assert!(actual.contains("bar"));
        assert!(!actual.contains("ZZZ"));

        let actual = file_contents(expected_stderr_file);
        assert!(actual.contains("ZZZ"));
        assert!(!actual.contains("bar"));
    })
}
