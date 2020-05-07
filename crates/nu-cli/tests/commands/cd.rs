use nu_test_support::fs::{Stub::EmptyFile, Stub::FileWithContent};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::path::PathBuf;

#[test]
fn filesystem_change_from_current_directory_using_relative_path() {
    Playground::setup("cd_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            r#"
                cd cd_test_1
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.test());
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

        assert_eq!(PathBuf::from(actual.out), dirs.formats());
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

        assert_eq!(PathBuf::from(actual.out), dirs.test().join("odin"));
    })
}

#[test]
fn filesytem_change_from_current_directory_using_relative_path_and_dash() {
    Playground::setup("cd_test_4", |dirs, sandbox| {
        sandbox.within("odin").mkdir("-");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd odin/-
                pwd | echo $it
            "#
        );

        assert_eq!(
            PathBuf::from(actual.out),
            dirs.test().join("odin").join("-")
        );
    })
}

#[test]
fn filesystem_change_current_directory_to_parent_directory() {
    Playground::setup("cd_test_5", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ..
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.root());
    })
}

#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots() {
    Playground::setup("cd_test_6", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                cd ...
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.test());
    })
}

#[test]
fn filesystem_change_current_directory_to_parent_directory_after_delete_cwd() {
    Playground::setup("cd_test_7", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                rm {}/foo/bar 
                echo ","
                cd .. 
                pwd | echo $it
            "#,
            dirs.test()
        );

        let actual = actual.out.split(',').nth(1).unwrap();

        assert_eq!(PathBuf::from(actual), *dirs.test().join("foo"));
    })
}

#[test]
fn filesystem_change_to_home_directory() {
    Playground::setup("cd_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ~
                pwd | echo $it
            "#
        );

        assert_eq!(Some(PathBuf::from(actual.out)), dirs::home_dir());
    })
}

#[test]
fn filesystem_change_to_a_directory_containing_spaces() {
    Playground::setup("cd_test_9", |dirs, sandbox| {
        sandbox.mkdir("robalino turner katz");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd "robalino turner katz"
                pwd | echo $it
            "#
        );

        assert_eq!(
            PathBuf::from(actual.out),
            dirs.test().join("robalino turner katz")
        );
    })
}

#[test]
fn filesystem_not_a_directory() {
    Playground::setup("cd_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("ferris_did_it.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "cd ferris_did_it.txt"
        );

        assert!(
            actual.err.contains("ferris_did_it.txt"),
            "actual={:?}",
            actual.err
        );
        assert!(
            actual.err.contains("is not a directory"),
            "actual={:?}",
            actual.err
        );
    })
}

#[test]
fn filesystem_directory_not_found() {
    Playground::setup("cd_test_11", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "cd dir_that_does_not_exist"

        );

        assert!(
            actual.err.contains("dir_that_does_not_exist"),
            "actual={:?}",
            actual.err
        );
        assert!(
            actual.err.contains("directory not found"),
            "actual={:?}",
            actual.err
        );
    })
}

#[test]
fn filesystem_change_directory_to_symlink_relative() {
    Playground::setup("cd_test_12", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("boo");
        sandbox.symlink("foo", "foo_link");

        let actual = nu!(
            cwd: dirs.test().join("boo"),
            r#"
                cd ../foo_link
                pwd | echo $it
            "#
        );

        assert_eq!(PathBuf::from(actual.out), dirs.test().join("foo"));
    })
}

#[test]
fn valuesystem_change_from_current_path_using_relative_path() {
    Playground::setup("cd_test_13", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [[bin]]
                    path = "src/plugins/turner.rs"

                    [[bin]]
                    path = "src/plugins/robalino.rs"

                    [[bin]]
                    path = "src/plugins/katz.rs"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd bin
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(PathBuf::from(actual.out), PathBuf::from("/bin"));
    })
}

#[test]
fn valuesystem_change_from_current_path_using_absolute_path() {
    Playground::setup("cd_test_14", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependencies]
                    turner-ts = "0.1.1"
                    robalino-tkd = "0.0.1"
                    katz-ember = "0.2.3"

                    [[bin]]
                    path = "src/plugins/arepa.rs"

                    [[bin]]
                    path = "src/plugins/bbq.rs"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd bin
                cd /dependencies
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(PathBuf::from(actual.out), PathBuf::from("/dependencies"));
    })
}

#[test]
fn valuesystem_switch_back_to_previous_working_path() {
    Playground::setup("cd_test_15", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependencies]
                    turner-ts = "0.1.1"
                    robalino-tkd = "0.0.1"
                    katz-ember = "0.2.3"
                    odin-gf = "0.2.1"

                    [[bin]]
                    path = "src/plugins/arepa.rs"

                    [[bin]]
                    path = "src/plugins/bbq.rs"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd dependencies
                cd /bin
                cd -
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(PathBuf::from(actual.out), PathBuf::from("/dependencies"));
    })
}

#[test]
fn valuesystem_change_from_current_path_using_relative_path_and_dash() {
    Playground::setup("cd_test_16", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContent(
                "sample.toml",
                r#"
                    [package]
                    - = ["Yehuda Katz <wycats@gmail.com>", "Jonathan Turner <jonathan.d.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]

                    [[bin]]
                    path = "src/plugins/arepa.rs"

                    [[bin]]
                    path = "src/plugins/bbq.rs"
                "#
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd package/-
                cd /bin
                cd -
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(PathBuf::from(actual.out), PathBuf::from("/package/-"));
    })
}

#[test]
fn valuesystem_change_current_path_to_parent_path() {
    Playground::setup("cd_test_17", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContent(
                "sample.toml",
                r#"
                    [package]
                    emberenios = ["Yehuda Katz <wycats@gmail.com>", "Jonathan Turner <jonathan.d.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                "#
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd package/emberenios
                cd ..
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(PathBuf::from(actual.out), PathBuf::from("/package"));
    })
}

#[test]
fn valuesystem_change_to_a_path_containing_spaces() {
    Playground::setup("cd_test_18", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    ["pa que te"]
                    el = "pollo loco"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                enter sample.toml
                cd "pa que te"
                pwd | echo $it
                exit
            "#
        );

        assert_eq!(
            PathBuf::from(actual.out),
            PathBuf::from("/").join("pa que te")
        );
    })
}

#[test]
fn valuesystem_path_not_found() {
    Playground::setup("cd_test_19", |dirs, _| {
        let actual = nu!(
            cwd: dirs.formats(),
            r#"
            enter cargo_sample.toml
            cd im_a_path_that_does_not_exist
            exit
        "#
        );

        assert!(actual.err.contains("Can not change to path inside"));
        assert!(actual.err.contains("No such path exists"));
    })
}
