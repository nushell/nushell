use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn lists_regular_files(playground: Playground) -> Result {
    playground.empty_file("andres.txt")?;
    playground.empty_file("jt.txt")?;
    playground.empty_file("yehuda.txt")?;

    test()
        .cwd(playground.path())
        .run("(ls).name")
        .expect_value_eq(["andres.txt", "jt.txt", "yehuda.txt"])
}

#[test]
fn lists_regular_files_using_asterisk_wildcard(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("(ls *.txt).name")
        .expect_value_eq(["amigos.txt", "los.txt", "tres.txt"])
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
fn lists_regular_files_using_question_mark_wildcard(playground: Playground) -> Result {
    playground.empty_file("yehuda.10.txt")?;
    playground.empty_file("jt.10.txt")?;
    playground.empty_file("andres.10.txt")?;
    playground.empty_file("chicken_not_to_be_picked_up.100.txt")?;

    test()
        .cwd(playground.path())
        .run("ls *.??.txt | length")
        .expect_value_eq(3)
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
fn fails_when_glob_doesnt_match(playground: Playground) -> Result {
    playground.empty_file("root1.txt")?;
    playground.empty_file("root2.txt")?;

    let err = test()
        .cwd(playground.path())
        .run("ls root3*")
        .expect_shell_error()?;
    let err_msg = err.generic_msg()?;
    assert_contains("file or folder not found", err_msg);

    Ok(())
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
fn let_typed_glob_expands_in_ls(playground: Playground) -> Result {
    playground.empty_file("a.toml")?;
    playground.empty_file("b.toml")?;
    playground.empty_file("c.txt")?;

    test()
        .cwd(playground.path())
        .run(r#"let g: glob = "*.toml"; ls $g | length"#)
        .expect_value_eq(2)
}

#[test]
fn let_into_glob_still_works_in_ls(playground: Playground) -> Result {
    playground.empty_file("a.toml")?;
    playground.empty_file("b.toml")?;
    playground.empty_file("c.txt")?;

    test()
        .cwd(playground.path())
        .run(r#"let g = "*.toml" | into glob; ls $g | length"#)
        .expect_value_eq(2)
}

#[test]
fn lists_hidden_file_when_explicitly_specified(playground: Playground) -> Result {
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file(".testdotfile")?;

    test()
        .cwd(playground.path())
        .run("ls .testdotfile | length")
        .expect_value_eq(1)
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
fn fails_with_permission_denied() -> Result {
    use nu_protocol::shell_error::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt;

    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox
            .within("dir_a")
            .with_files(&[EmptyFile("yehuda.11.txt"), EmptyFile("jt10.txt")]);

        let is_root = nix::unistd::Uid::effective().is_root();
        let dir_a = dirs.test().join("dir_a");
        let original_permissions = std::fs::metadata(&dir_a)?.permissions();

        let mut permissions = original_permissions.clone();
        permissions.set_mode(0o000);
        std::fs::set_permissions(&dir_a, permissions)?;
        let path_arg_result: Result<Value> = test().cwd(dirs.test()).run("ls dir_a");

        let mut permissions = original_permissions.clone();
        permissions.set_mode(0o100);
        std::fs::set_permissions(&dir_a, permissions)?;
        let cwd_result: Result<Value> = test().cwd(&dir_a).run("ls");

        std::fs::set_permissions(&dir_a, original_permissions)?;

        if !is_root {
            let path_arg_err = path_arg_result.expect_io_error()?;
            let cwd_err = cwd_result.expect_io_error()?;

            assert_matches!(
                path_arg_err.kind,
                ErrorKind::Std(std::io::ErrorKind::PermissionDenied, ..)
            );
            assert_matches!(
                cwd_err.kind,
                ErrorKind::Std(std::io::ErrorKind::PermissionDenied, ..)
            );
        }

        Ok(())
    })
}

#[test]
fn lists_files_including_starting_with_dot(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;
    playground.empty_file(".hidden1.txt")?;
    playground.empty_file(".hidden2.txt")?;

    test()
        .cwd(playground.path())
        .run("ls -a | length")
        .expect_value_eq(5)
}

#[test]
fn list_all_columns(playground: Playground) -> Result {
    playground.empty_file("Leonardo.yaml")?;
    playground.empty_file("Raphael.json")?;
    playground.empty_file("Donatello.xml")?;
    playground.empty_file("Michelangelo.txt")?;

    // Normal Operation
    test()
        .cwd(playground.path())
        .run("ls | columns")
        .expect_value_eq(["name", "type", "size", "modified"])?;
    // Long
    let expected = cfg_select! {
        unix => [
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
        ],
        windows => [
            "name", "type", "target", "readonly", "size", "created", "accessed", "modified",
        ],
    };
    test()
        .cwd(playground.path())
        .run("ls -l | columns")
        .expect_value_eq(expected)
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
fn list_a_directory_not_exists(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run("ls a_directory_not_exists")
        .expect_error_code_eq("nu::shell::io::not_found")
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
#[deps(NU)]
fn list_directory_contains_invalid_utf8(playground: Playground) -> Result {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let v: [u8; 4] = [7, 196, 144, 188];
    let s = OsStr::from_bytes(&v);

    let cwd = playground.path();
    let path = cwd.join(s);

    std::fs::create_dir_all(path).expect("failed to create directory");

    // unfortunately `ls` prints warning on stdout for this
    let result: CompleteResult = test().cwd(cwd).run("nu -n -c 'ls' | complete")?;

    assert_contains("warning: get non-utf8 filename", result.stdout);
    assert_contains("No matches found for", result.stderr);

    Ok(())
}

#[test]
fn list_ignores_ansi(playground: Playground) -> Result {
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;

    // asserting no errors are raised
    let _: Value = test()
        .cwd(playground.path())
        .run("ls | find .txt | each {|| ls $in.name }")?;

    Ok(())
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
fn list_flag_false(playground: Playground) -> Result {
    // Check that ls flags respect explicit values
    playground.empty_file(".hidden")?;
    playground.empty_file("normal")?;
    playground.empty_file("another_normal")?;

    // TODO Remove this cfg value when we have an OS-agnostic way
    // of creating hidden files using the playground.
    #[cfg(unix)]
    {
        test()
            .cwd(playground.path())
            .run("ls --all=false | length")
            .expect_value_eq(2)?;
    }

    test()
        .cwd(playground.path())
        .run("ls --long=false | columns | length")
        .expect_value_eq(4)?;

    test()
        .cwd(playground.path())
        .run("ls --full-paths=false | get name | any { $in =~ / }")
        .expect_value_eq(false)?;

    Ok(())
}

#[test]
fn list_empty_string(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;

    test()
        .cwd(playground.path())
        .run("ls ''")
        .expect_error_code_eq("nu::shell::io::not_found")
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
fn list_with_multiple_path(playground: Playground) -> Result {
    playground.empty_file("f1.txt")?;
    playground.empty_file("f2.txt")?;
    playground.empty_file("f3.txt")?;

    test()
        .cwd(playground.path())
        .run("(ls f1.txt f2.txt).name")
        .expect_value_eq(["f1.txt", "f2.txt"])?;

    // report errors if one path not exists
    test()
        .cwd(playground.path())
        .run("ls asdf f1.txt")
        .expect_error_code_eq("nu::shell::io::not_found")?;

    // ls with spreading empty list should returns nothing.
    test()
        .cwd(playground.path())
        .run("ls ...[]")
        .expect_value_eq([(); 0])?;

    Ok(())
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
fn list_symlink_with_full_path(playground: Playground) -> Result {
    playground.empty_file("test_file.txt")?;

    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink("test_file.txt", playground.path().join("test_link1"));
    #[cfg(windows)]
    let _ =
        std::os::windows::fs::symlink_file("test_file.txt", playground.path().join("test_link1"));

    test()
        .cwd(playground.path())
        .run("(ls -l test_link1).target.0")
        .expect_value_eq("test_file.txt")?;

    test()
        .cwd(playground.path())
        .run("(ls -lf test_link1).target.0")
        .expect_value_eq(playground.path().join("test_file.txt").to_string_lossy())?;

    Ok(())
}

#[test]
fn consistent_list_order(playground: Playground) -> Result {
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;

    let no_arg: Value = test().cwd(playground.path()).run("ls")?;
    let with_arg: Value = test().cwd(playground.path()).run("ls .")?;

    assert_eq!(no_arg, with_arg);

    Ok(())
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
fn ls_literal_empty_directory(playground: Playground) -> Result {
    playground.dir("emptydir")?;

    test()
        .cwd(dirs.root())
        .run("ls ls_literal_empty_dir_dc/emptydir | length")
        .expect_value_eq(0)
        .expect("ls literal empty directory should not error with dc-glob");

    Ok(())
}

/// Regression for https://github.com/nushell/nushell/issues/18600#issuecomment-5077246342
///
/// `ls **` goes through `glob_from`, which rewrites the pattern to absolute
/// `{cwd}/**`. Multi-component trailing `**` must expand (directories only).
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_bare_double_star() -> Result {
    Playground::setup("ls_dc_bare_double_star", |dirs, sandbox| {
        sandbox.mkdir("1");
        sandbox.mkdir("1/2");
        sandbox.mkdir("1/2/3");
        sandbox.within("1/2/3").with_files(&[EmptyFile("file.txt")]);
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");

        // Nested directories present; file.txt must not appear (trailing ** is dir-only).
        // Must not error with "No matches found".
        // Compare expanded paths so relative vs absolute display names both work.
        let code = "
            let paths = (ls ** | get name | each { path expand } | sort)
            (
                ($paths | length) >= 5
                and ($paths | any {|p| $p | str ends-with $'(char psep)1'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2(char psep)3'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
                and not ($paths | any {|p| $p | str ends-with 'file.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("ls ** should list nested dirs and not files with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn ls_dc_glob_prefixed_trailing_double_star() -> Result {
    Playground::setup("ls_dc_prefixed_trailing", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");
        sandbox
            .within("foo")
            .with_files(&[EmptyFile("sibling.txt")]);
        sandbox
            .within("foo/bar")
            .with_files(&[EmptyFile("nested.txt")]);

        let code = "
            let paths = (ls foo/** | get name | each { path expand } | sort)
            (
                ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
                and not ($paths | any {|p| $p | str ends-with 'sibling.txt'})
                and not ($paths | any {|p| $p | str ends-with 'nested.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("ls foo/** should list foo and nested dirs only with dc-glob");
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
