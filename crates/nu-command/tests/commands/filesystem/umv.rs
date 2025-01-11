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
                ls test_folder\test_folder_on_x\test_file_on_x.txt | length
            "#
        );
        assert_eq!(_actual.out, "1");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                x:test_folder_on_x\
                cd -
                mv X:test_file_on_x.txt x:mv.txt
                ls test_folder\test_folder_on_x\mv.txt | length
            "#
        );
        assert_eq!(_actual.out, "1");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst X: /D | touch out
            "#
        );
    })
}
