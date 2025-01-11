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
                subst R: /D | touch out
                subst R: test_folder
                r:
                mkdir test_folder_on_r
                cd -
                r:test_folder_on_r\
                touch test_file_on_r.txt
                cd -
                ls test_folder\test_folder_on_r\test_file_on_r.txt | length
            "#
        );
        assert_eq!(_actual.out, "1");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                r:test_folder_on_r\
                cd -
                mv R:test_file_on_r.txt r:mv.txt
                ls test_folder\test_folder_on_r\mv.txt | length
            "#
        );
        assert_eq!(_actual.out, "1");
        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst R: /D | touch out
            "#
        );
    })
}
