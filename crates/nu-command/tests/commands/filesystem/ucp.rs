#[cfg(windows)]
use nu_test_support::{nu, playground::Playground};

#[cfg(windows)]
#[test]
fn test_pwd_per_drive() {
    Playground::setup("test_cp_pwd_per_drive", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst T: /D | touch out
                subst T: test_folder
                t:
                mkdir test_folder_on_t
                cd -
                t:test_folder_on_t\
                touch test_file_on_t.txt
                cd -
                cp test_folder\test_folder_on_t\test_file_on_t.txt t:test_folder_on_t\cp.txt
            "#
        );
        assert!(_actual.err.is_empty());
        let expected_file = dirs.test().join(r"test_folder\test_folder_on_t\cp.txt");
        assert!(expected_file.exists());
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst T: /D | touch out
            "#
        );
    })
}
