mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;
use std::path::{Path, PathBuf};

#[test]
fn rm_removes_a_file() {
    Playground::setup("rm_regular_file_test", |dirs, playground| {
        playground
            .with_files(vec![EmptyFile("i_will_be_deleted.txt")])
            .test_dir_name();

        nu!(dirs.root(), "rm rm_regular_file_test/i_will_be_deleted.txt");

        let path = dirs.test().join("i_will_be_deleted.txt");

        assert!(!h::file_exists_at(path));
    })
}

#[test]
fn rm_removes_files_with_wildcard() {
    Playground::setup("rm_wildcard_test_1", |dirs, playground| {
        playground
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
            ])
            .test_dir_name();

        nu!(dirs.test(), r#"rm "src/*/*/*.rs""#);

        assert!(!h::files_exist_at(
            vec![
                "src/parser/parse/token_tree.rs",
                "src/parser/hir/baseline_parse.rs",
                "src/parser/hir/baseline_parse_tokens.rs"
            ],
            dirs.test()
        ));

        assert_eq!(
            Playground::glob_vec(&format!("{}/src/*/*/*.rs", dirs.test().display())),
            Vec::<PathBuf>::new()
        );
    })
}

#[test]
fn rm_removes_deeply_nested_directories_with_wildcard_and_recursive_flag() {
    Playground::setup("rm_wildcard_test_2", |dirs, playground| {
        playground
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
            ])
            .test_dir_name();

        nu!(dirs.test(), "rm src/* --recursive");

        assert!(!h::files_exist_at(
            vec!["src/parser/parse", "src/parser/hir"],
            dirs.test()
        ));
    })
}

#[test]
fn rm_removes_directory_contents_without_recursive_flag_if_empty() {
    Playground::setup("rm_directory_removal_recursively_test_1", |dirs, _| {
        nu!(dirs.root(), "rm rm_directory_removal_recursively_test_1");

        assert!(!h::file_exists_at(dirs.test()));
    })
}

#[test]
fn rm_removes_directory_contents_with_recursive_flag() {
    Playground::setup(
        "rm_directory_removal_recursively_test_2",
        |dirs, playground| {
            playground
                .with_files(vec![
                    EmptyFile("yehuda.txt"),
                    EmptyFile("jonathan.txt"),
                    EmptyFile("andres.txt"),
                ])
                .test_dir_name();

            nu!(
                dirs.root(),
                "rm rm_directory_removal_recursively_test_2 --recursive"
            );

            assert!(!h::file_exists_at(dirs.test()));
        },
    )
}

#[test]
fn rm_errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag() {
    Playground::setup(
        "rm_prevent_directory_removal_without_flag_test",
        |dirs, playground| {
            playground
                .with_files(vec![EmptyFile("some_empty_file.txt")])
                .test_dir_name();

            let output = nu_error!(
                dirs.root(),
                "rm rm_prevent_directory_removal_without_flag_test"
            );

            assert!(h::file_exists_at(dirs.test()));
            assert!(output.contains("is a directory"));
        },
    )
}

#[test]
fn rm_errors_if_attempting_to_delete_single_dot_as_argument() {
    Playground::setup(
        "rm_errors_if_attempting_to_delete_single_dot_as_argument",
        |dirs, _| {
            let output = nu_error!(dirs.root(), "rm .");

            assert!(output.contains("may not be removed"));
        },
    )
}

#[test]
fn rm_errors_if_attempting_to_delete_two_dot_as_argument() {
    Playground::setup(
        "rm_errors_if_attempting_to_delete_single_dot_as_argument",
        |dirs, _| {
            let output = nu_error!(dirs.root(), "rm ..");

            assert!(output.contains("may not be removed"));
        },
    )
}
