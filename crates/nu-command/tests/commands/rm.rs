use nu_test_support::fs::{files_exist_at, Stub::EmptyFile};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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
            "rm -r src/*"
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
        let actual = nu!(
            cwd: dirs.root(),
            "rm rm_test_6"
        );

        assert!(dirs.test().exists());
        assert!(actual.err.contains("cannot remove non-empty directory"));
    })
}

#[test]
fn errors_if_attempting_to_delete_single_dot_as_argument() {
    Playground::setup("rm_test_7", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm ."
        );

        assert!(actual.err.contains("cannot remove any parent directory"));
    })
}

#[test]
fn errors_if_attempting_to_delete_two_dot_as_argument() {
    Playground::setup("rm_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm .."
        );

        assert!(actual.err.contains("cannot"));
    })
}

#[test]
fn removes_multiple_directories() {
    Playground::setup("rm_test_9", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(vec![EmptyFile("a.rs"), EmptyFile("b.rs")])
            .within("src/cli")
            .with_files(vec![EmptyFile("c.rs"), EmptyFile("d.rs")])
            .within("test")
            .with_files(vec![EmptyFile("a_test.rs"), EmptyFile("b_test.rs")]);

        nu!(
            cwd: dirs.test(),
            "rm src test --recursive"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_multiple_files() {
    Playground::setup("rm_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "rm yehuda.txt jonathan.txt andres.txt"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_multiple_files_with_asterisks() {
    Playground::setup("rm_test_11", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.toml"),
        ]);

        nu!(
            cwd: dirs.test(),
            "rm *.txt *.toml"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn allows_doubly_specified_file() {
    Playground::setup("rm_test_12", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("yehuda.txt"), EmptyFile("jonathan.toml")]);

        let actual = nu!(
            cwd: dirs.test(),
            "rm *.txt yehuda* *.toml"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        assert!(!actual.out.contains("error"))
    })
}

#[test]
fn remove_files_from_two_parents_up_using_multiple_dots_and_glob() {
    Playground::setup("rm_test_13", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        nu!(
            cwd: dirs.test().join("foo/bar"),
            "rm .../*.txt"
        );

        assert!(!files_exist_at(
            vec!["yehuda.txt", "jonathan.txt", "kevin.txt"],
            dirs.test()
        ));
    })
}

#[test]
fn no_errors_if_attempting_to_delete_non_existent_file_with_f_flag() {
    Playground::setup("rm_test_14", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm -f non_existent_file.txt"
        );

        assert!(!actual.err.contains("no valid path"));
    })
}

#[test]
fn rm_wildcard_keeps_dotfiles() {
    Playground::setup("rm_test_15", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("foo"), EmptyFile(".bar")]);

        nu!(
            cwd: dirs.test(),
            r#"rm *"#
        );

        assert!(!files_exist_at(vec!["foo"], dirs.test()));
        assert!(files_exist_at(vec![".bar"], dirs.test()));
    })
}

#[test]
fn rm_wildcard_leading_dot_deletes_dotfiles() {
    Playground::setup("rm_test_16", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("foo"), EmptyFile(".bar")]);

        nu!(
            cwd: dirs.test(),
            r#"rm .*"#
        );

        assert!(files_exist_at(vec!["foo"], dirs.test()));
        assert!(!files_exist_at(vec![".bar"], dirs.test()));
    })
}

#[test]
fn removes_files_with_case_sensitive_glob_matches_by_default() {
    Playground::setup("glob_test", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("A0"), EmptyFile("a1")]);

        nu!(
            cwd: dirs.root(),
            "rm glob_test/A*"
        );

        let deleted_path = dirs.test().join("A0");
        let skipped_path = dirs.test().join("a1");

        assert!(!deleted_path.exists());
        assert!(skipped_path.exists());
    })
}

#[test]
fn remove_ignores_ansi() {
    Playground::setup("rm_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("test.txt")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "ls | find test | get name | rm $in.0; ls",
        );
        assert!(actual.out.is_empty());
    });
}
