#[cfg(windows)]
use nu_test_support::{nu, playground::Playground};

#[cfg(windows)]
#[test]
fn test_pwd_per_drive() {
    Playground::setup("test_mv_pwd_per_drive", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst X: /D | touch out
                subst X: test_folder
                x:
                mkdir test_folder_on_x
                cd -
                x:test_folder_on_x\
                touch test_file_on_x.txt
                cd -
            "#
        );
        assert!(_actual.err.is_empty());
        let expected_file = dirs
            .test()
            .join("test_folder\\test_folder_on_x\\test_file_on_x.txt");
        assert!(expected_file.exists());
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                mv X:test_file_on_x.txt x:mv.txt
            "#
        );
        assert!(!expected_file.exists());
        eprintln!("StdOut: {}", _actual.out);
        let expected_file = dirs.test().join("test_folder\\test_folder_on_x\\mv.txt");
        assert!(expected_file.exists());
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst X: /D | touch out
            "#
        );
    })
}
