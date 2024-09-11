use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn checks_if_existing_file_exists() {
    Playground::setup("path_exists_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "echo spam.txt | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_if_missing_file_exists() {
    Playground::setup("path_exists_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo spam.txt | path exists"
        );

        assert_eq!(actual.out, "false");
    })
}

#[test]
fn checks_if_dot_exists() {
    Playground::setup("path_exists_3", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo '.' | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_if_double_dot_exists() {
    Playground::setup("path_exists_4", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo '..' | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_tilde_relative_path_exists() {
    let actual = nu!("'~' | path exists");
    assert_eq!(actual.out, "true");
}

#[test]
fn const_path_exists() {
    let actual = nu!("const exists = ('~' | path exists); $exists");
    assert_eq!(actual.out, "true");
}

#[test]
fn path_exists_under_a_non_directory() {
    Playground::setup("path_exists_6", |dirs, _| {
        let actual = nu!(
                cwd: dirs.test(),
                "touch test_file; 'test_file/aaa' | path exists"
        );
        assert_eq!(actual.out, "false");
        assert!(actual.err.is_empty());
    })
}

#[test]
fn test_check_symlink_exists() {
    use nu_test_support::{nu, playground::Playground};

    let symlink_target = "symlink_target";
    let symlink = "symlink";
    Playground::setup("path_exists_5", |dirs, sandbox| {
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(dirs.test().join(symlink_target), dirs.test().join(symlink))
            .unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            dirs.test().join(symlink_target),
            dirs.test().join(symlink),
        )
        .unwrap();

        let false_out = "false".to_string();
        let shell_res = nu!(cwd: sandbox.cwd(), "'symlink_target' | path exists");
        assert_eq!(false_out, shell_res.out);
        let shell_res = nu!(cwd: sandbox.cwd(), "'symlink' | path exists");
        assert_eq!(false_out, shell_res.out);
        let shell_res = nu!(cwd: sandbox.cwd(), "'symlink' | path exists -n");
        assert_eq!("true".to_string(), shell_res.out);
    });
}
