use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn lists_regular_files() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | length
        ");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls *.txt
            | length
        ");

        assert_eq!(actual.out, "3");
    })
}

#[cfg(not(target_os = "windows"))]
#[test]
fn lists_regular_files_in_special_folder() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox
            .mkdir("[abcd]")
            .mkdir("[bbcd]")
            .mkdir("abcd]")
            .mkdir("abcd")
            .mkdir("abcd/*")
            .mkdir("abcd/?")
            .with_files(&[EmptyFile("[abcd]/test.txt")])
            .with_files(&[EmptyFile("abcd]/test.txt")])
            .with_files(&[EmptyFile("abcd/*/test.txt")])
            .with_files(&[EmptyFile("abcd/?/test.txt")])
            .with_files(&[EmptyFile("abcd/?/test2.txt")]);

        let actual = nu!(
            cwd: dirs.test().join("abcd]"), format!(r#"ls | length"#));
        assert_eq!(actual.out, "1");
        let actual = nu!(
            cwd: dirs.test(), format!(r#"ls abcd] | length"#));
        assert_eq!(actual.out, "1");
        let actual = nu!(
            cwd: dirs.test().join("[abcd]"), format!(r#"ls | length"#));
        assert_eq!(actual.out, "1");
        let actual = nu!(
            cwd: dirs.test().join("[bbcd]"), format!(r#"ls | length"#));
        assert_eq!(actual.out, "0");
        let actual = nu!(
            cwd: dirs.test().join("abcd/*"), format!(r#"ls | length"#));
        assert_eq!(actual.out, "1");
        let actual = nu!(
            cwd: dirs.test().join("abcd/?"), format!(r#"ls | length"#));
        assert_eq!(actual.out, "2");
        let actual = nu!(
            cwd: dirs.test().join("abcd/*"), format!(r#"ls -D ../* | length"#));
        assert_eq!(actual.out, "2");
        let actual = nu!(
            cwd: dirs.test().join("abcd/*"), format!(r#"ls ../* | length"#));
        assert_eq!(actual.out, "2");
        let actual = nu!(
            cwd: dirs.test().join("abcd/?"), format!(r#"ls -D ../* | length"#));
        assert_eq!(actual.out, "2");
        let actual = nu!(
            cwd: dirs.test().join("abcd/?"), format!(r#"ls ../* | length"#));
        assert_eq!(actual.out, "2");
    })
}

#[rstest::rstest]
#[case("j?.??.txt", 1)]
#[case("j????.txt", 2)]
#[case("?????.txt", 3)]
#[case("????c.txt", 1)]
#[case("ye??da.10.txt", 1)]
#[case("yehuda.?0.txt", 1)]
#[case("??????.10.txt", 2)]
#[case("[abcd]????.txt", 1)]
#[case("??[ac.]??.txt", 3)]
#[case("[ab]bcd/??.txt", 2)]
#[case("?bcd/[xy]y.txt", 2)]
#[case("?bcd/[xy]y.t?t", 2)]
#[case("[[]abcd[]].txt", 1)]
#[case("[[]?bcd[]].txt", 2)]
#[case("??bcd[]].txt", 2)]
#[case("??bcd].txt", 2)]
#[case("[[]?bcd].txt", 2)]
#[case("[[]abcd].txt", 1)]
#[case("[[][abcd]bcd[]].txt", 2)]
#[case("'[abcd].txt'", 1)]
#[case("'[bbcd].txt'", 1)]
fn lists_regular_files_using_question_mark(#[case] command: &str, #[case] expected: usize) {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.mkdir("abcd").mkdir("bbcd").with_files(&[
            EmptyFile("abcd/xy.txt"),
            EmptyFile("bbcd/yy.txt"),
            EmptyFile("[abcd].txt"),
            EmptyFile("[bbcd].txt"),
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jt.10.txt"),
            EmptyFile("jtabc.txt"),
            EmptyFile("abcde.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), format!(r#"ls {command} | length"#));
        assert_eq!(actual.out, expected.to_string());
    })
}

#[test]
fn lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jt.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls *.??.txt
            | length
        ");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_all_files_in_directories_from_stream() {
    Playground::setup("ls_test_4", |dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile("root1.txt"), EmptyFile("root2.txt")])
            .within("dir_a")
            .with_files(&[EmptyFile("yehuda.10.txt"), EmptyFile("jt10.txt")])
            .within("dir_b")
            .with_files(&[
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            ]);

        let actual = nu!(cwd: dirs.test(), "
            echo dir_a dir_b
            | each { |it| ls $it }
            | flatten | length
        ");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn does_not_fail_if_glob_matches_empty_directory() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.within("dir_a");

        let actual = nu!(cwd: dirs.test(), "
            ls dir_a
            | length
        ");

        assert_eq!(actual.out, "0");
    })
}

#[test]
fn fails_when_glob_doesnt_match() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("root1.txt"), EmptyFile("root2.txt")]);

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
        sandbox.with_files(&[
            EmptyFile("yahuda.yaml"),
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            "
                ls ... | length
            "
        );

        assert_eq!(actual.out, "5");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"ls ... | sort-by name | get name.0 | str replace -a '\' '/'"#
        );
        assert_eq!(actual.out, "../../andres.xml");
    })
}

#[test]
fn lists_hidden_file_when_explicitly_specified() {
    Playground::setup("ls_test_7", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile(".testdotfile"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls .testdotfile
            | length
        ");

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn lists_all_hidden_files_when_glob_contains_dot() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox
            .with_files(&[
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(&[
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jt10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within("dir_b")
            .with_files(&[
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(cwd: dirs.test(), "
            ls **/.*
            | length
        ");

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
            .with_files(&[
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(&[
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jt10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within(".dir_b")
            .with_files(&[
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(cwd: dirs.test(), "
            ls **/*
            | length
        ");

        assert_eq!(actual.out, "5");
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn glob_with_hidden_directory() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox.within(".dir_b").with_files(&[
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            EmptyFile(".dotfile3"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls **/*
            | length
        ");

        assert_eq!(actual.out, "");
        assert!(actual.err.contains("No matches found"));

        // will list files if provide `-a` flag.
        let actual = nu!(cwd: dirs.test(), "
            ls -a **/*
            | length
        ");

        assert_eq!(actual.out, "4");
    })
}

#[test]
#[cfg(unix)]
fn fails_with_permission_denied() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox
            .within("dir_a")
            .with_files(&[EmptyFile("yehuda.11.txt"), EmptyFile("jt10.txt")]);

        let actual_with_path_arg = nu!(cwd: dirs.test(), "
            chmod 000 dir_a; ls dir_a
        ");

        let actual_in_cwd = nu!(cwd: dirs.test(), "
            chmod 100 dir_a; cd dir_a; ls
        ");

        let get_uid = nu!(cwd: dirs.test(), "
            id -u
        ");
        let is_root = get_uid.out == "0";

        assert!(actual_with_path_arg.err.contains("Permission denied") || is_root);

        assert!(actual_in_cwd.err.contains("Permission denied") || is_root);
    })
}

#[test]
fn lists_files_including_starting_with_dot() {
    Playground::setup("ls_test_9", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
            EmptyFile(".hidden1.txt"),
            EmptyFile(".hidden2.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls -a
            | length
        ");

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn list_all_columns() {
    Playground::setup("ls_test_all_columns", |dirs, sandbox| {
        sandbox.with_files(&[
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
                    "user",
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
            .with_files(&[EmptyFile("nushell.json")])
            .within("dir_empty");
        let actual = nu!(cwd: dirs.test(), r#"
            cd dir_empty;
            ['.' '././.' '..' '../dir_files' '../dir_files/*']
            | each { |it| ls --directory ($it | into glob) }
            | flatten
            | get name
            | to text
        "#);
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
            .with_files(&[EmptyFile("nushell.json")])
            .within("dir_empty");
        // Test if there are some files in the current directory
        let actual = nu!(cwd: dirs.test(), "
            cd dir_files;
            ls --directory
            | get name
            | to text
        ");
        let expected = ".";
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls --directory (-D)"
        );
        // Test if there is no file in the current directory
        let actual = nu!(cwd: dirs.test(), "
            cd dir_empty;
            ls -D
            | get name
            | to text
        ");
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
    let file_type = nu!(cwd: "C:\\Windows\\System32", r#"ls Configuration* | where name == "Configuration" | get type.0"#);
    assert_eq!(file_type.out, "dir");

    let file_size = nu!(cwd: "C:\\Windows\\System32", r#"ls Configuration* | where name == "Configuration" | get size.0"#);
    assert_ne!(file_size.out.trim(), "");

    let file_modified = nu!(cwd: "C:\\Windows\\System32", r#"ls Configuration* | where name == "Configuration" | get modified.0"#);
    assert_ne!(file_modified.out.trim(), "");

    let file_accessed = nu!(cwd: "C:\\Windows\\System32", r#"ls -l Configuration* | where name == "Configuration" | get accessed.0"#);
    assert_ne!(file_accessed.out.trim(), "");

    let file_created = nu!(cwd: "C:\\Windows\\System32", r#"ls -l Configuration* | where name == "Configuration" | get created.0"#);
    assert_ne!(file_created.out.trim(), "");

    let ls_with_filter = nu!(cwd: "C:\\Windows\\System32", "ls | where size > 10mb");
    assert_eq!(ls_with_filter.err, "");
}

#[test]
fn list_a_directory_not_exists() {
    Playground::setup("ls_test_directory_not_exists", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "ls a_directory_not_exists");
        assert!(actual.err.contains("nu::shell::io::not_found"));
    })
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
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
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls | find .txt | each {|| ls $in.name }
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn list_unknown_long_flag() {
    let actual = nu!("ls --full-path");

    assert!(actual.err.contains("Did you mean: `--full-paths`?"));
}

#[test]
fn list_unknown_short_flag() {
    let actual = nu!("ls -r");

    assert!(actual.err.contains("Use `--help` to see available flags"));
}

#[test]
fn list_flag_false() {
    // Check that ls flags respect explicit values
    Playground::setup("ls_test_false_flag", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile(".hidden"),
            EmptyFile("normal"),
            EmptyFile("another_normal"),
        ]);

        // TODO Remove this cfg value when we have an OS-agnostic way
        // of creating hidden files using the playground.
        #[cfg(unix)]
        {
            let actual = nu!(cwd: dirs.test(), "
            ls --all=false | length
                        ");

            assert_eq!(actual.out, "2");
        }

        let actual = nu!(cwd: dirs.test(), "
            ls --long=false | columns | length
        ");

        assert_eq!(actual.out, "4");

        let actual = nu!(cwd: dirs.test(), "
            ls --full-paths=false | get name | any { $in =~ / }
        ");

        assert_eq!(actual.out, "false");
    })
}

#[test]
fn list_empty_string() {
    Playground::setup("ls_empty_string", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("yehuda.txt")]);

        let actual = nu!(cwd: dirs.test(), "ls ''");
        assert!(actual.err.contains("does not exist"));
    })
}

#[test]
fn list_with_tilde() {
    Playground::setup("ls_tilde", |dirs, sandbox| {
        sandbox
            .within("~tilde")
            .with_files(&[EmptyFile("f1.txt"), EmptyFile("f2.txt")]);

        let actual = nu!(cwd: dirs.test(), "ls '~tilde'");
        assert!(actual.out.contains("f1.txt"));
        assert!(actual.out.contains("f2.txt"));
        assert!(actual.out.contains("~tilde"));
        let actual = nu!(cwd: dirs.test(), "ls ~tilde");
        assert!(actual.err.contains("nu::shell::io::not_found"));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde'; ls $f");
        assert!(actual.out.contains("f1.txt"));
        assert!(actual.out.contains("f2.txt"));
        assert!(actual.out.contains("~tilde"));
    })
}

#[test]
fn list_with_multiple_path() {
    Playground::setup("ls_multiple_path", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "ls f1.txt f2.txt");
        assert!(actual.out.contains("f1.txt"));
        assert!(actual.out.contains("f2.txt"));
        assert!(!actual.out.contains("f3.txt"));
        assert!(actual.status.success());

        // report errors if one path not exists
        let actual = nu!(cwd: dirs.test(), "ls asdf f1.txt");
        assert!(actual.err.contains("nu::shell::io::not_found"));
        assert!(!actual.status.success());

        // ls with spreading empty list should returns nothing.
        let actual = nu!(cwd: dirs.test(), "ls ...[] | length");
        assert_eq!(actual.out, "0");
    })
}

#[test]
fn list_inside_glob_metachars_dir() {
    Playground::setup("list_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[EmptyFile("test_file.txt")]);

        let actual = nu!(
            cwd: dirs.test().join(sub_dir),
            "ls test_file.txt | get name.0 | path basename",
        );
        assert!(actual.out.contains("test_file.txt"));
    });
}

#[test]
fn list_inside_tilde_glob_metachars_dir() {
    Playground::setup(
        "list_files_inside_tilde_glob_metachars_dir",
        |dirs, sandbox| {
            let sub_dir = "~test[]";
            sandbox
                .within(sub_dir)
                .with_files(&[EmptyFile("test_file.txt")]);

            // need getname.0 | path basename because the output path
            // might be too long to output as a single line.
            let actual = nu!(
                cwd: dirs.test().join(sub_dir),
                "ls test_file.txt | get name.0 | path basename",
            );
            assert!(actual.out.contains("test_file.txt"));

            let actual = nu!(
                cwd: dirs.test(),
                "ls '~test[]' | get name.0 | path basename"
            );
            assert!(actual.out.contains("test_file.txt"));
        },
    );
}

#[test]
fn list_symlink_with_full_path() {
    Playground::setup("list_symlink_with_full_path", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);

        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink("test_file.txt", dirs.test().join("test_link1"));
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_file("test_file.txt", dirs.test().join("test_link1"));
        let actual = nu!(
            cwd: dirs.test(),
            "ls -l test_link1 | get target.0"
        );
        assert_eq!(actual.out, "test_file.txt");
        let actual = nu!(
            cwd: dirs.test(),
            "ls -lf test_link1 | get target.0"
        );
        assert_eq!(
            actual.out,
            dirs.test().join("test_file.txt").to_string_lossy()
        );
    })
}

#[test]
fn consistent_list_order() {
    Playground::setup("ls_test_order", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let no_arg = nu!(cwd: dirs.test(), "ls");

        let with_arg = nu!(cwd: dirs.test(), "ls .");

        assert_eq!(no_arg.out, with_arg.out);
    })
}
