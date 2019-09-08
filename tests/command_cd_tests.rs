mod helpers;

use helpers::Playground;
use std::path::PathBuf;

#[test]
fn filesytem_change_from_current_directory_using_relative_path() {
    Playground::setup("cd_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            r#"
                cd cd_test_1
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual), *dirs.test());
    })
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path() {
    Playground::setup("cd_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd {}
                pwd | echo $it
            "#,
            dirs.formats()
        );

        assert_eq!(PathBuf::from(actual), dirs.formats());
    }) 
}

#[test]
fn filesystem_switch_back_to_previous_working_directory() {
    Playground::setup("cd_test_3", |dirs, sandbox| {
        sandbox.mkdir("odin");

        let actual = nu!(
            cwd: dirs.test().join("odin"),
            r#"
                cd {}
                cd -
                pwd | echo $it
            "#,
            dirs.test()
        );

        assert_eq!(PathBuf::from(actual), dirs.test().join("odin"));
    })
}

#[test]
fn filesystem_change_current_directory_to_parent_directory() {
    Playground::setup("cd_test_4", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ..
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual), *dirs.root());
    })
}

#[test]
fn file_system_change_to_home_directory() {
    Playground::setup("cd_test_5", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ~
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual), dirs::home_dir().unwrap());
    })
}

#[test]
fn filesystem_change_to_a_directory_containing_spaces() {
    Playground::setup("cd_test_6", |dirs, sandbox| {
        sandbox.mkdir("robalino turner katz");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd "robalino turner katz"
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual), dirs.test().join("robalino turner katz"));
    })
}

#[test]
fn filesystem_directory_not_found() {
    let actual = nu_error!(
    	cwd: "tests/fixtures",
    	"cd dir_that_does_not_exist"
    );

    assert!(actual.contains("dir_that_does_not_exist"));
    assert!(actual.contains("directory not found"));
}
