use nu_test_support::fs::{files_exist_at, Stub::EmptyFile};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error};

#[test]
fn removes_a_file() {
    Playground::setup("rm_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("i_will_be_deleted.txt")]);

        nu!(
            cwd: dirs.root(),
            "rm rm_test_1/i_will_be_deleted.txt"
        );

        let path = dirs.test().join("i_will_be_deleted.txt");

        assert!(!path.exists());
    })
}

#[test]
fn removes_files_with_wildcard() {
    Playground::setup("rm_test_2", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(vec![
                EmptyFile("cli.rs"),
                EmptyFile("lib.rs"),
                EmptyFile("prelude.rs"),
            ])
            .within("src/parser")
            .with_files(vec![EmptyFile("parse.rs"), EmptyFile("parser.rs")])
            .within("src/parser/parse")
            .with_files(vec![EmptyFile("token_tree.rs")])
            .within("src/parser/hir")
            .with_files(vec![
                EmptyFile("baseline_parse.rs"),
                EmptyFile("baseline_parse_tokens.rs"),
            ]);

        nu!(
            cwd: dirs.test(),
            r#"rm "src/*/*/*.rs""#
        );

        assert!(!files_exist_at(
            vec![
                "src/parser/parse/token_tree.rs",
                "src/parser/hir/baseline_parse.rs",
                "src/parser/hir/baseline_parse_tokens.rs"
            ],
            dirs.test()
        ));

        assert_eq!(
            Playground::glob_vec(&format!("{}/src/*/*/*.rs", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_deeply_nested_directories_with_wildcard_and_recursive_flag() {
    Playground::setup("rm_test_3", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(vec![
                EmptyFile("cli.rs"),
                EmptyFile("lib.rs"),
                EmptyFile("prelude.rs"),
            ])
            .within("src/parser")
            .with_files(vec![EmptyFile("parse.rs"), EmptyFile("parser.rs")])
            .within("src/parser/parse")
            .with_files(vec![EmptyFile("token_tree.rs")])
            .within("src/parser/hir")
            .with_files(vec![
                EmptyFile("baseline_parse.rs"),
                EmptyFile("baseline_parse_tokens.rs"),
            ]);

        nu!(
            cwd: dirs.test(),
            "rm src/* --recursive"
        );

        assert!(!files_exist_at(
            vec!["src/parser/parse", "src/parser/hir"],
            dirs.test()
        ));
    })
}

#[test]
fn removes_directory_contents_without_recursive_flag_if_empty() {
    Playground::setup("rm_test_4", |dirs, _| {
        nu!(
            cwd: dirs.root(),
            "rm rm_test_4"
        );

        assert!(!dirs.test().exists());
    })
}

#[test]
fn removes_directory_contents_with_recursive_flag() {
    Playground::setup("rm_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        nu!(
            cwd: dirs.root(),
            "rm rm_test_5 --recursive"
        );

        assert!(!dirs.test().exists());
    })
}

#[test]
fn errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag() {
    Playground::setup("rm_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("some_empty_file.txt")]);

        let actual = nu_error!(
            cwd: dirs.root(),
            "rm rm_test_6"
        );

        assert!(dirs.test().exists());
        assert!(actual.contains("is a directory"));
    })
}

#[test]
fn errors_if_attempting_to_delete_single_dot_as_argument() {
    Playground::setup("rm_test_7", |dirs, _| {
        let actual = nu_error!(
            cwd: dirs.root(),
            "rm ."
        );

        assert!(actual.contains("may not be removed"));
    })
}

#[test]
fn errors_if_attempting_to_delete_two_dot_as_argument() {
    Playground::setup("rm_test_8", |dirs, _| {
        let actual = nu_error!(
            cwd: dirs.root(),
            "rm .."
        );

        assert!(actual.contains("may not be removed"));
    })
}
