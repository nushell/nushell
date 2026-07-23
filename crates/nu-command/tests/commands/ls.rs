use nu_protocol::ParseError;
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn lists_regular_files() -> Result {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("andres.txt"),
            EmptyFile("jt.txt"),
            EmptyFile("yehuda.txt"),
        ]);

        test().cwd(dirs.test()).run("(ls).name").expect_value_eq([
            "andres.txt",
            "jt.txt",
            "yehuda.txt",
        ])
    })
}

#[test]
fn lists_regular_files_using_asterisk_wildcard() -> Result {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("(ls *.txt).name")
            .expect_value_eq(["amigos.txt", "los.txt", "tres.txt"])
    })
}

#[cfg(not(target_os = "windows"))]
#[test]
fn lists_regular_files_in_special_folder() -> Result {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox
            .mkdir("[abcd]")
            .mkdir("[bbcd]")
            .mkdir("abcd]")
            .mkdir("abcd")
            .mkdir("abcd/*")
            .mkdir("abcd/?")
            .with_files(&[
                EmptyFile("[abcd]/test.txt"),
                EmptyFile("abcd]/test.txt"),
                EmptyFile("abcd/*/test.txt"),
                EmptyFile("abcd/?/test.txt"),
                EmptyFile("abcd/?/test2.txt"),
            ]);

        test()
            .cwd(dirs.test().join("abcd]"))
            .run("(ls).name")
            .expect_value_eq(["test.txt"])?;

        // Quote the path: `]` is a list closer and cannot appear unquoted inside
        // a parenthesized subexpression.
        test()
            .cwd(dirs.test())
            .run(r#"(ls "abcd]").name"#)
            .expect_value_eq(["abcd]/test.txt"])?;

        test()
            .cwd(dirs.test().join("[abcd]"))
            .run("(ls).name")
            .expect_value_eq(["test.txt"])?;

        test()
            .cwd(dirs.test().join("[bbcd]"))
            .run("ls")
            .expect_value_eq([(); 0])?;

        test()
            .cwd(dirs.test().join("abcd/*"))
            .run("(ls).name")
            .expect_value_eq(["test.txt"])?;

        test()
            .cwd(dirs.test().join("abcd/?"))
            .run("(ls).name")
            .expect_value_eq(["test.txt", "test2.txt"])?;

        test()
            .cwd(dirs.test().join("abcd/*"))
            .run("ls -D ../* | length")
            .expect_value_eq(2)?;

        test()
            .cwd(dirs.test().join("abcd/*"))
            .run("ls ../* | length")
            .expect_value_eq(2)?;

        test()
            .cwd(dirs.test().join("abcd/?"))
            .run("ls -D ../* | length")
            .expect_value_eq(2)?;

        test()
            .cwd(dirs.test().join("abcd/?"))
            .run("ls ../* | length")
            .expect_value_eq(2)?;

        Ok(())
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
fn lists_regular_files_using_question_mark(#[case] ls_arg: &str, #[case] expected: i64) -> Result {
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

        test()
            .cwd(dirs.test())
            .run(format!("ls {ls_arg} | length"))
            .expect_value_eq(expected)
    })
}

#[test]
fn lists_regular_files_using_question_mark_wildcard() -> Result {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jt.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls *.??.txt | length")
            .expect_value_eq(3)
    })
}

#[test]
fn lists_all_files_in_directories_from_stream() -> Result {
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

        let code = "
            echo dir_a dir_b
            | each { |it| ls $it }
            | flatten
            | length
        ";
        test().cwd(dirs.test()).run(code).expect_value_eq(4)
    })
}

#[test]
fn does_not_fail_if_glob_matches_empty_directory() -> Result {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.within("dir_a");

        test()
            .cwd(dirs.test())
            .run("ls dir_a | length")
            .expect_value_eq(0)
    })
}

#[test]
fn fails_when_glob_doesnt_match() -> Result {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("root1.txt"), EmptyFile("root2.txt")]);

        let err = test()
            .cwd(dirs.test())
            .run("ls root3*")
            .expect_shell_error()?;
        let err_msg = err.generic_msg()?;
        assert_contains("file or folder not found", err_msg);

        Ok(())
    })
}

#[test]
fn list_files_from_two_parents_up_using_multiple_dots() -> Result {
    Playground::setup("ls_test_6", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yahuda.yaml"),
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        test()
            .cwd(dirs.test().join("foo/bar"))
            .run("ls ... | length")
            .expect_value_eq(5)?;

        test()
            .cwd(dirs.test().join("foo/bar"))
            .run(r#"ls ... | sort-by name | get name.0 | str replace -a '\' '/'"#)
            .expect_value_eq("../../andres.xml")
    })
}

#[test]
fn let_typed_glob_expands_in_ls() -> Result {
    Playground::setup("ls_let_glob_expand", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("a.toml"), EmptyFile("b.toml"), EmptyFile("c.txt")]);

        test()
            .cwd(dirs.test())
            .run(r#"let g: glob = "*.toml"; ls $g | length"#)
            .expect_value_eq(2)
    })
}

#[test]
fn let_into_glob_still_works_in_ls() -> Result {
    Playground::setup("ls_into_glob_regression", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("a.toml"), EmptyFile("b.toml"), EmptyFile("c.txt")]);

        test()
            .cwd(dirs.test())
            .run(r#"let g = "*.toml" | into glob; ls $g | length"#)
            .expect_value_eq(2)
    })
}

#[test]
fn lists_hidden_file_when_explicitly_specified() -> Result {
    Playground::setup("ls_test_7", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile(".testdotfile"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls .testdotfile | length")
            .expect_value_eq(1)
    })
}

#[test]
fn lists_all_hidden_files_when_glob_contains_dot() -> Result {
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

        test()
            .cwd(dirs.test())
            .run("ls **/.* | length")
            .expect_value_eq(3)
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn lists_all_hidden_files_when_glob_does_not_contain_dot() -> Result {
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

        test()
            .cwd(dirs.test())
            .run("ls **/* | length")
            .expect_value_eq(5)
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn glob_with_hidden_directory() -> Result {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox.within(".dir_b").with_files(&[
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            EmptyFile(".dotfile3"),
        ]);

        let err = test()
            .cwd(dirs.test())
            .run("ls **/* | length")
            .expect_shell_error()?;
        let err_msg = err.generic_msg()?;
        assert_contains("file or folder not found", err_msg);

        // will list files if provide `-a` flag.
        test()
            .cwd(dirs.test())
            .run("ls -a **/* | length")
            .expect_value_eq(4)
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
fn lists_files_including_starting_with_dot() -> Result {
    Playground::setup("ls_test_9", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
            EmptyFile(".hidden1.txt"),
            EmptyFile(".hidden2.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls -a | length")
            .expect_value_eq(5)
    })
}

#[test]
fn list_all_columns() -> Result {
    Playground::setup("ls_test_all_columns", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("Leonardo.yaml"),
            EmptyFile("Raphael.json"),
            EmptyFile("Donatello.xml"),
            EmptyFile("Michelangelo.txt"),
        ]);

        // Normal Operation
        test()
            .cwd(dirs.test())
            .run("ls | columns")
            .expect_value_eq(["name", "type", "size", "modified"])?;
        // Long
        let expected = cfg_select! {
            unix => {
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
            }
            windows => {
                [
                    "name",
                    "type",
                    "target",
                    "readonly",
                    "size",
                    "created",
                    "accessed",
                    "modified",
                ]
            }
        };
        test()
            .cwd(dirs.test())
            .run("ls -l | columns")
            .expect_value_eq(expected)
    })
}

#[test]
fn lists_with_directory_flag() -> Result {
    Playground::setup("ls_test_flag_directory_1", |dirs, sandbox| {
        sandbox
            .within("dir_files")
            .with_files(&[EmptyFile("nushell.json")])
            .within("dir_empty");

        let code = "
            ['.' '././.' '..' '../dir_files' '../dir_files/*']
            | each { |it| ls --directory ($it | into glob) }
            | flatten
            | get name
        ";
        let expected = [".", ".", "..", "../dir_files", "../dir_files/nushell.json"];
        #[cfg(windows)]
        let expected = expected.map(|e| e.replace('/', "\\"));

        test()
            .cwd(dirs.test().join("dir_empty"))
            .run(code)
            .expect_value_eq(expected)
    })
}

#[test]
fn lists_with_directory_flag_without_argument() -> Result {
    Playground::setup("ls_test_flag_directory_2", |dirs, sandbox| {
        sandbox
            .within("dir_files")
            .with_files(&[EmptyFile("nushell.json")])
            .within("dir_empty");

        // Test if there are some files in the current directory
        test()
            .cwd(dirs.test().join("dir_files"))
            .run("ls --directory | get name")
            .expect_value_eq(["."])?;

        // Test if there is no file in the current directory
        test()
            .cwd(dirs.test().join("dir_empty"))
            .run("ls -D | get name")
            .expect_value_eq(["."])?;

        Ok(())
    })
}

/// Rust's fs::metadata function is unable to read info for certain system files on Windows,
/// like the `C:\Windows\System32\Configuration` folder. https://github.com/rust-lang/rust/issues/96980
/// This test confirms that Nu can work around this successfully.
#[test]
#[cfg(windows)]
fn can_list_system_folder() -> Result {
    // the awkward `ls Configuration* | where name == "Configuration"` thing is for speed;
    // listing the entire System32 folder is slow and `ls Configuration*` alone
    // might return more than 1 file someday

    let code = r#"
        ls -l Configuration*
        | where name == "Configuration"
        | first -s 
        | select name type size modified accessed created
    "#;
    let out: nu_protocol::Record = test().cwd("C:\\Windows\\System32").run(code)?;

    assert_eq!(out["name"].as_str().unwrap(), "Configuration");
    assert_eq!(out["type"].as_str().unwrap(), "dir");

    let _ = out["size"].as_filesize()?;
    let _ = out["modified"].as_date()?;
    let _ = out["accessed"].as_date()?;
    let _ = out["created"].as_date()?;

    let _: Value = test()
        .cwd("C:\\Windows\\System32")
        .run("ls | where size > 10mb")?;

    Ok(())
}

#[test]
fn list_a_directory_not_exists() -> Result {
    Playground::setup("ls_test_directory_not_exists", |dirs, _sandbox| {
        test()
            .cwd(dirs.test())
            .run("ls a_directory_not_exists")
            .expect_error_code_eq("nu::shell::io::not_found")
    })
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
#[deps(NU)]
fn list_directory_contains_invalid_utf8() -> Result {
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

            // unfortunately `ls` prints warning on stdout for this
            let result: CompleteResult = test().cwd(cwd).run("nu -n -c 'ls' | complete")?;

            assert_contains("warning: get non-utf8 filename", result.stdout);
            assert_contains("No matches found for", result.stderr);

            Ok(())
        },
    )
}

#[test]
fn list_ignores_ansi() -> Result {
    Playground::setup("ls_test_ansi", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        // asserting no errors are raised
        let _: Value = test()
            .cwd(dirs.test())
            .run("ls | find .txt | each {|| ls $in.name }")?;

        Ok(())
    })
}

#[test]
fn list_unknown_long_flag() -> Result {
    let err = test().run("ls --full-path").expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::UnknownFlag(_, _, _, help) if help == "Did you mean: `--full-paths`?"
    );
    Ok(())
}

#[test]
fn list_unknown_short_flag() -> Result {
    let err = test().run("ls -r").expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::UnknownFlag(_, _, _, help) if help == "Use `--help` to see available flags"
    );
    Ok(())
}

#[test]
fn list_flag_false() -> Result {
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
            test()
                .cwd(dirs.test())
                .run("ls --all=false | length")
                .expect_value_eq(2)?;
        }

        test()
            .cwd(dirs.test())
            .run("ls --long=false | columns | length")
            .expect_value_eq(4)?;

        test()
            .cwd(dirs.test())
            .run("ls --full-paths=false | get name | any { $in =~ / }")
            .expect_value_eq(false)?;

        Ok(())
    })
}

#[test]
fn list_empty_string() -> Result {
    Playground::setup("ls_empty_string", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("yehuda.txt")]);

        test()
            .cwd(dirs.test())
            .run("ls ''")
            .expect_error_code_eq("nu::shell::io::not_found")
    })
}

#[test]
fn list_with_tilde() -> Result {
    Playground::setup("ls_tilde", |dirs, sandbox| {
        sandbox
            .within("~tilde")
            .with_files(&[EmptyFile("f1.txt"), EmptyFile("f2.txt")]);

        test()
            .cwd(dirs.test())
            .run("(ls '~tilde').name")
            .expect_value_eq(cfg_select! {
                unix => ["~tilde/f1.txt", "~tilde/f2.txt"],
                windows => ["~tilde\\f1.txt", "~tilde\\f2.txt"],
            })?;

        test()
            .cwd(dirs.test())
            .run("ls ~tilde")
            .expect_error_code_eq("nu::shell::io::not_found")?;

        // pass variable
        test()
            .cwd(dirs.test())
            .run("let f = '~tilde'; (ls $f).name")
            .expect_value_eq(cfg_select! {
                unix => ["~tilde/f1.txt", "~tilde/f2.txt"],
                windows => ["~tilde\\f1.txt", "~tilde\\f2.txt"],
            })?;

        Ok(())
    })
}

#[test]
fn list_with_multiple_path() -> Result {
    Playground::setup("ls_multiple_path", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("(ls f1.txt f2.txt).name")
            .expect_value_eq(["f1.txt", "f2.txt"])?;

        // report errors if one path not exists
        test()
            .cwd(dirs.test())
            .run("ls asdf f1.txt")
            .expect_error_code_eq("nu::shell::io::not_found")?;

        // ls with spreading empty list should returns nothing.
        test()
            .cwd(dirs.test())
            .run("ls ...[]")
            .expect_value_eq([(); 0])?;

        Ok(())
    })
}

#[test]
fn list_inside_glob_metachars_dir() -> Result {
    Playground::setup("list_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[EmptyFile("test_file.txt")]);

        test()
            .cwd(dirs.test().join(sub_dir))
            .run("(ls test_file.txt).name.0 | path basename")
            .expect_value_eq("test_file.txt")
    })
}

#[test]
fn list_inside_tilde_glob_metachars_dir() -> Result {
    Playground::setup(
        "list_files_inside_tilde_glob_metachars_dir",
        |dirs, sandbox| {
            let sub_dir = "~test[]";
            sandbox
                .within(sub_dir)
                .with_files(&[EmptyFile("test_file.txt")]);

            // need name.0 | path basename because the output path
            // might be too long to output as a single line.
            test()
                .cwd(dirs.test().join(sub_dir))
                .run("(ls test_file.txt).name.0 | path basename")
                .expect_value_eq("test_file.txt")?;

            test()
                .cwd(dirs.test())
                .run("(ls '~test[]').name.0 | path basename")
                .expect_value_eq("test_file.txt")?;

            Ok(())
        },
    )
}

#[test]
fn list_symlink_with_full_path() -> Result {
    Playground::setup("list_symlink_with_full_path", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);

        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink("test_file.txt", dirs.test().join("test_link1"));
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_file("test_file.txt", dirs.test().join("test_link1"));

        test()
            .cwd(dirs.test())
            .run("(ls -l test_link1).target.0")
            .expect_value_eq("test_file.txt")?;

        test()
            .cwd(dirs.test())
            .run("(ls -lf test_link1).target.0")
            .expect_value_eq(dirs.test().join("test_file.txt").to_string_lossy())?;

        Ok(())
    })
}

#[test]
fn consistent_list_order() -> Result {
    Playground::setup("ls_test_order", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let no_arg: Value = test().cwd(dirs.test()).run("ls")?;
        let with_arg: Value = test().cwd(dirs.test()).run("ls .")?;

        assert_eq!(no_arg, with_arg);

        Ok(())
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_literal_prefix_wildcard() -> Result {
    Playground::setup("ls_dc_literal_prefix_wildcard", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox.within("subdir").with_files(&[
            EmptyFile("nu_test1"),
            EmptyFile("nu_test2"),
            EmptyFile("other"),
        ]);

        // Unquoted glob patterns (bare words) parse as Expand
        test()
            .cwd(dirs.test())
            .run("ls subdir/nu* | length")
            .expect_value_eq(2)
            .expect("ls subdir/nu* should list both nu_test files with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_literal_prefix_wildcard_metadata_populated() -> Result {
    Playground::setup("ls_dc_literal_prefix_meta", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("nu_test.txt")]);

        test()
            .cwd(dirs.test())
            .run("ls subdir/nu* | get type.0")
            .expect_value_eq("file")
            .expect("ls subdir/nu* should populate type column with dc-glob");

        test()
            .cwd(dirs.test())
            .run("ls subdir/nu* | get size.0 | into int")
            .expect_value_eq(0)
            .expect("ls subdir/nu* should populate size column with dc-glob");

        // modified column should be "datetime", not "nothing", when metadata is available
        test()
            .cwd(dirs.test())
            .run("ls subdir/nu* | get modified.0 | describe")
            .expect_value_eq("datetime")
            .expect("ls subdir/nu* should populate modified column with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_wildcard_then_literal() -> Result {
    Playground::setup("ls_dc_wildcard_literal", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox.within("subdir").with_files(&[
            EmptyFile("nu_test1"),
            EmptyFile("nu_test2"),
            EmptyFile("other"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls subdir/*nu* | length")
            .expect_value_eq(2)
            .expect("ls subdir/*nu* should list both nu_test files with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_wildcard_then_literal_metadata_populated() -> Result {
    Playground::setup("ls_dc_wildcard_literal_meta", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("nu_test.txt")]);

        test()
            .cwd(dirs.test())
            .run("ls subdir/*nu* | get type.0")
            .expect_value_eq("file")
            .expect("ls subdir/*nu* should populate type column with dc-glob");

        test()
            .cwd(dirs.test())
            .run("ls subdir/*nu* | get size.0 | into int")
            .expect_value_eq(0)
            .expect("ls subdir/*nu* should populate size column with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_literal_directory() -> Result {
    Playground::setup("ls_literal_dir_dc", |dirs, sandbox| {
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("test.txt")]);

        test()
            .cwd(dirs.root())
            .run("ls ls_literal_dir_dc/subdir | length")
            .expect_value_eq(1)
            .expect("ls literal directory should list its contents with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_literal_empty_directory() -> Result {
    Playground::setup("ls_literal_empty_dir_dc", |dirs, sandbox| {
        sandbox.mkdir("emptydir");

        test()
            .cwd(dirs.root())
            .run("ls ls_literal_empty_dir_dc/emptydir | length")
            .expect_value_eq(0)
            .expect("ls literal empty directory should not error with dc-glob");
    });

    Ok(())
}

// Windows does not allow `*` in filenames, so this regression only applies on Unix.
#[cfg(not(windows))]
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_with_file_named_star_lists_all_entries() -> Result {
    // Regression for #18631: with dc-glob, a file named `*` must not hide
    // every other entry when `ls` expands the default `*` pattern.
    // Use distinct names that stay unique on case-insensitive filesystems.
    Playground::setup("ls_file_named_star_dc", |dirs, sandbox| {
        sandbox
            .with_files(&[
                EmptyFile("file_a"),
                EmptyFile("file_b"),
                EmptyFile("file_c"),
                EmptyFile("*"),
            ])
            .mkdir("dir_a")
            .mkdir("dir_b")
            .mkdir("dir_c");

        test()
            .cwd(dirs.test())
            .run("ls | length")
            .expect_value_eq(7)?;

        test()
            .cwd(dirs.test())
            .run("ls * | length")
            .expect_value_eq(7)?;

        Ok(())
    })
}
