use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn lists_regular_files() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.txt
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jonathan.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.??.txt
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_all_files_in_directories_from_stream() {
    Playground::setup("ls_test_4", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("root1.txt"), EmptyFile("root2.txt")])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
            ])
            .within("dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo dir_a dir_b
                | each { |it| ls $it }
                | flatten | length
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn does_not_fail_if_glob_matches_empty_directory() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.within("dir_a");

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls dir_a
                | length
            "#
        ));

        assert_eq!(actual.out, "0");
    })
}

#[test]
fn fails_when_glob_doesnt_match() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("root1.txt"), EmptyFile("root2.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls root3*"
        );

        assert!(actual.err.contains("no matches found"));
    })
}

#[test]
fn list_files_from_two_parents_up_using_multiple_dots() {
    Playground::setup("ls_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yahuda.yaml"),
            EmptyFile("jonathan.json"),
            EmptyFile("andres.xml"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                ls ... | length
            "#
        );

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn lists_hidden_file_when_explicitly_specified() {
    Playground::setup("ls_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile(".testdotfile"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls .testdotfile
                | length
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn lists_all_hidden_files_when_glob_contains_dot() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within("dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls **/.*
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn lists_all_hidden_files_when_glob_does_not_contain_dot() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within(".dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls **/*
                | length
            "#
        ));

        assert_eq!(actual.out, "5");
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn glob_with_hidden_directory() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox.within(".dir_b").with_files(vec![
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            EmptyFile(".dotfile3"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls **/*
                | length
            "#
        ));

        assert_eq!(actual.out, "");
        assert!(actual.err.contains("No matches found"));

        // will list files if provide `-a` flag.
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls -a **/*
                | length
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
#[cfg(unix)]
fn fails_with_ls_to_dir_without_permission() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.within("dir_a").with_files(vec![
            EmptyFile("yehuda.11.txt"),
            EmptyFile("jonathan.10.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                chmod 000 dir_a; ls dir_a
            "#
        ));

        let check_not_root = nu!(
            cwd: dirs.test(), pipeline(
                r#"
                    id -u
                "#
        ));

        assert!(
            actual
                .err
                .contains("The permissions of 0 do not allow access for this user")
                || check_not_root.out == "0"
        );
    })
}

#[test]
fn lists_files_including_starting_with_dot() {
    Playground::setup("ls_test_9", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
            EmptyFile(".hidden1.txt"),
            EmptyFile(".hidden2.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls -a
                | length
            "#
        ));

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn list_all_columns() {
    Playground::setup("ls_test_all_columns", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("Leonardo.yaml"),
            EmptyFile("Raphael.json"),
            EmptyFile("Donatello.xml"),
            EmptyFile("Michelangelo.txt"),
        ]);
        // Normal Operation
        let actual = nu!(
            cwd: dirs.test(),
            "ls | columns | to md"
        );
        let expected = ["name", "type", "size", "modified"].join("");
        assert_eq!(actual.out, expected, "column names are incorrect for ls");
        // Long
        let actual = nu!(
            cwd: dirs.test(),
            "ls -l | columns | to md"
        );
        let expected = {
            #[cfg(unix)]
            {
                [
                    "name",
                    "type",
                    "target",
                    "readonly",
                    "mode",
                    "num_links",
                    "inode",
                    "uid",
                    "group",
                    "size",
                    "created",
                    "accessed",
                    "modified",
                ]
                .join("")
            }

            #[cfg(windows)]
            {
                [
                    "name", "type", "target", "readonly", "size", "created", "accessed", "modified",
                ]
                .join("")
            }
        };
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls long"
        );
    });
}

#[test]
fn lists_with_directory_flag() {
    Playground::setup("ls_test_flag_directory_1", |dirs, sandbox| {
        sandbox
            .within("dir_files")
            .with_files(vec![EmptyFile("nushell.json")])
            .within("dir_empty");
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                cd dir_empty;
                ['.' '././.' '..' '../dir_files' '../dir_files/*']
                | each { |it| ls --directory $it }
                | flatten
                | get name
                | to text
            "#
        ));
        let expected = [".", ".", "..", "../dir_files", "../dir_files/nushell.json"].join("");
        #[cfg(windows)]
        let expected = expected.replace('/', "\\");
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls --directory (-D)"
        );
    });
}

#[test]
fn lists_with_directory_flag_without_argument() {
    Playground::setup("ls_test_flag_directory_2", |dirs, sandbox| {
        sandbox
            .within("dir_files")
            .with_files(vec![EmptyFile("nushell.json")])
            .within("dir_empty");
        // Test if there are some files in the current directory
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                cd dir_files;
                ls --directory
                | get name
                | to text
            "#
        ));
        let expected = ".";
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls --directory (-D)"
        );
        // Test if there is no file in the current directory
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                cd dir_empty;
                ls -D
                | get name
                | to text
            "#
        ));
        let expected = ".";
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls --directory (-D)"
        );
    });
}

/// Rust's fs::metadata function is unable to read info for certain system files on Windows,
/// like the `C:\Windows\System32\Configuration` folder. https://github.com/rust-lang/rust/issues/96980
/// This test confirms that Nu can work around this successfully.
#[test]
#[cfg(windows)]
fn can_list_system_folder() {
    // the awkward `ls Configuration* | where name == "Configuration"` thing is for speed;
    // listing the entire System32 folder is slow and `ls Configuration*` alone
    // might return more than 1 file someday
    let file_type = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls Configuration* | where name == "Configuration" | get type.0"#
    ));
    assert_eq!(file_type.out, "dir");

    let file_size = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls Configuration* | where name == "Configuration" | get size.0"#
    ));
    assert!(file_size.out.trim() != "");

    let file_modified = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls Configuration* | where name == "Configuration" | get modified.0"#
    ));
    assert!(file_modified.out.trim() != "");

    let file_accessed = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls -l Configuration* | where name == "Configuration" | get accessed.0"#
    ));
    assert!(file_accessed.out.trim() != "");

    let file_created = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls -l Configuration* | where name == "Configuration" | get created.0"#
    ));
    assert!(file_created.out.trim() != "");

    let ls_with_filter = nu!(
        cwd: "C:\\Windows\\System32", pipeline(
        r#"ls | where size > 10mb"#
    ));
    assert_eq!(ls_with_filter.err, "");
}

#[test]
fn list_a_directory_not_exists() {
    Playground::setup("ls_test_directory_not_exists", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "ls a_directory_not_exists");
        assert!(actual.err.contains("directory not found"));
    })
}

#[cfg(target_os = "linux")]
#[test]
fn list_directory_contains_invalid_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    Playground::setup(
        "ls_test_directory_contains_invalid_utf8",
        |dirs, _sandbox| {
            let v: [u8; 4] = [7, 196, 144, 188];
            let s = OsStr::from_bytes(&v);

            let cwd = dirs.test();
            let path = cwd.join(s);

            std::fs::create_dir_all(path).expect("failed to create directory");

            let actual = nu!(cwd: cwd, "ls");

            assert!(actual.out.contains("warning: get non-utf8 filename"));
            assert!(actual.err.contains("No matches found for"));
        },
    )
}

#[test]
fn list_ignores_ansi() {
    Playground::setup("ls_test_ansi", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls | find .txt | each { ls $in.name } 
            "#
        ));

        assert!(actual.err.is_empty());
    })
}
