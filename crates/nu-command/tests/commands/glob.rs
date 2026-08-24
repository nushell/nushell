use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::path::{Path, PathBuf};

#[test]
fn empty_glob_pattern_triggers_error(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;

    let err = test()
        .cwd(playground.path())
        .run("glob ''")
        .expect_shell_error()?;

    assert_contains("must not be empty", err.to_string());
    Ok(())
}

#[test]
fn nonempty_glob_lists_matching_paths(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '*' | length")
        .expect_value_eq(3)
}

#[test]
fn glob_subdirs() -> Result {
    Playground::setup("glob_subdirs", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob '**/*' | length")
            .expect_value_eq(8)
    })
}

#[test]
fn glob_subdirs_ignore_dirs() -> Result {
    Playground::setup("glob_subdirs_ignore_directories", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob '**/*' -D | length")
            .expect_value_eq(6)
    })
}

#[test]
fn glob_ignore_files() -> Result {
    Playground::setup("glob_ignore_files", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob '*' -F | length")
            .expect_value_eq(1)
    })
}

// clone of fs::create_file_at removing the parent panic, whose purpose I do not grok.
pub fn create_file_at(full_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let full_path = full_path.as_ref();
    std::fs::write(full_path, b"fake data")
}

// playground has root directory and subdirectories foo and foo/bar to play with
// specify all test files relative to root directory.
// OK to use fwd slash in paths, they're hacked to OS dir separator when needed (windows)
#[rstest]
#[case(".", "'*z'", &["ablez", "baker", "charliez"], &["ablez", "charliez"], "simple glob")]
#[case(".", "'qqq'", &["ablez", "baker", "charliez"], &[], "glob matches none")]
#[case("foo/bar", r"'*[\]}]*'", &["foo/bar/ab}le", "foo/bar/baker", "foo/bar/cha]rlie"], &["foo/bar/ab}le", "foo/bar/cha]rlie"], "glob has quoted metachars")]
#[case("foo/bar", "'../*'", &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/bar/../able", "foo/bar/../bar", "foo/bar/../charlie"], "glob matches files in parent")]
#[case("foo", "'./{a,b}*'", &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/able", "foo/bar"], "glob with leading ./ matches peer files")]
fn glob_files_in_parent(
    #[case] wd: &str,
    #[case] glob: &str,
    #[case] ini: &[&str],
    #[case] exp: &[&str],
    #[case] tag: &str,
) {
    Playground::setup("glob_test", |dirs, sandbox| {
        sandbox.within("foo").within("bar");
        let working_directory = &dirs.test().join(wd);

        for f in ini {
            create_file_at(dirs.test().join(f)).expect("couldn't create file");
        }

        let actual: String = test()
            .cwd(working_directory)
            .run(format!(r#"glob {glob} | sort | str join " ""#))
            .expect("glob should list matching paths");

        let mut expected: Vec<String> = vec![];
        for e in exp {
            // Normalize windows paths by converting / to \ and resolving .. components lexically.
            #[cfg(windows)]
            let e = {
                let mut path = PathBuf::new();
                for c in Path::new(e).components() {
                    if c == std::path::Component::ParentDir {
                        path.pop();
                    } else {
                        path.push(c)
                    }
                }
                path
            };
            #[cfg(not(windows))]
            let e = PathBuf::from(e);

            expected.push(dirs.test().join(e).to_string_lossy().to_string());
        }

        let expected = expected.join(" ");
        assert_eq!(actual, expected, "\n  test: {tag}");
    });
}

#[test]
fn glob_follow_symlinks() -> Result {
    Playground::setup("glob_follow_symlinks", |dirs, sandbox| {
        // Create a directory with some files
        sandbox.mkdir("target_dir");
        sandbox
            .within("target_dir")
            .with_files(&[EmptyFile("target_file.txt")]);

        let target_dir = dirs.test().join("target_dir");
        let symlink_path = dirs.test().join("symlink_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_dir, &symlink_path).expect("Failed to create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(target_dir, &symlink_path)
            .expect("Failed to create symlink");

        // on some systems/filesystems, symlinks are followed by default
        // on others (like Linux /sys), they aren't
        // Test that with the --follow-symlinks flag, files are found for sure
        test()
            .cwd(dirs.test())
            .run("glob 'symlink_dir/*.txt' --follow-symlinks | length")
            .expect_value_eq(1)
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_supports_depth_and_exclude(playground: Playground) -> Result {
    playground.empty_file("three.txt")?;
    playground.empty_file("four.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '*.txt' --depth 1 --exclude [four.txt] | length")
        .expect_value_eq(1)
        .expect("glob should support --depth and --exclude with dc-glob enabled");;

    Ok(())
}

/// Regression tests for https://github.com/nushell/nushell/issues/18600
///
/// With dc-glob:
/// - bare `**` lists directories (including the start dir), not files
/// - `**/*` lists files and dirs under the start, **not** the start itself
/// - extra `/*` segments enforce a minimum path depth
/// - `foo/**` includes the prefix directory itself (directories only)
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_recursive_depth_semantics() -> Result {
    Playground::setup("glob_dc_recursive_depth", |dirs, sandbox| {
        sandbox.mkdir("1");
        sandbox.mkdir("1/2");
        sandbox.mkdir("1/2/3");
        sandbox.within("1/2/3").with_files(&[EmptyFile("file.txt")]);
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");
        sandbox
            .within("foo")
            .with_files(&[EmptyFile("sibling.txt")]);

        // **/*: concrete membership — four entries under start, not the start itself
        let code = "
            let root = (pwd | path expand)
            let paths = (glob '**/*' | each { path expand } | sort)
            (
                ($paths | length) == 7
                and not ($paths | any {|p| $p == $root })
                and ($paths | any {|p| $p | str ends-with $'(char psep)1'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2(char psep)3'})
                and ($paths | any {|p| $p | str ends-with 'file.txt'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
                and ($paths | any {|p| $p | str ends-with 'sibling.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("**/* should list nested dirs/files under start, not the start dir");

        // **/*/*/* → min depth 3 under the 1/ tree (and deeper under foo if any)
        // With fixture: 1/2/3, 1/2/3/file.txt only at depth >= 3 from root for `1` tree.
        // Overall tree also has foo/bar — depth 2 only. So still just 2 matches from 1/.
        let code = "
            let paths = (glob '**/*/*/*' | each { path expand } | sort)
            (
                ($paths | length) == 2
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2(char psep)3'})
                and ($paths | any {|p| $p | str ends-with 'file.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("**/*/*/* should only match depth >= 3 paths");

        // bare ** → directories only (start + 1 + 1/2 + 1/2/3 + foo + foo/bar)
        let code = "
            let root = (pwd | path expand)
            let paths = (glob '**' | each { path expand } | sort)
            (
                ($paths | length) == 6
                and ($paths | any {|p| $p == $root })
                and ($paths | any {|p| $p | str ends-with $'(char psep)1'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2(char psep)3'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
                and not ($paths | any {|p| $p | str ends-with 'file.txt'})
                and not ($paths | any {|p| $p | str ends-with 'sibling.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("bare ** should list start + nested dirs only, not files");

        // foo/** includes foo itself and foo/bar, not files
        let code = "
            let paths = (glob 'foo/**' | each { path expand } | sort)
            (
                ($paths | length) == 2
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
                and not ($paths | any {|p| $p | str ends-with 'sibling.txt'})
            )
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(true)
            .expect("foo/** should include foo and nested dirs, not files");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_supports_follow_symlinks() -> Result {
    Playground::setup("glob_dc_follow_symlink", |dirs, sandbox| {
        sandbox.mkdir("target_dir");
        sandbox
            .within("target_dir")
            .with_files(&[EmptyFile("target_file.txt")]);

        let target_dir = dirs.test().join("target_dir");
        let symlink_path = dirs.test().join("symlink_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_dir, &symlink_path).expect("Failed to create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(target_dir, &symlink_path)
            .expect("Failed to create symlink");

        test()
            .cwd(dirs.test())
            .run("glob 'symlink_dir/*.txt' --follow-symlinks | length")
            .expect_value_eq(1)
            .expect("glob should follow symlinked dirs when --follow-symlinks is set");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_debug_subcommands_enabled(playground: Playground) -> Result {
    playground.empty_file("file.txt")?;

    test()
        .cwd(playground.path())
        .run("glob --dbg-parse '*' | str contains 'Wildcard'")
        .expect_value_eq(true)
        .expect("--dbg-parse should be available with dc-glob enabled");

    test()
        .cwd(playground.path())
        .run("glob --dbg-compile '**/*' | str contains 'Complete'")
        .expect_value_eq(true)
        .expect("--dbg-compile should be available with dc-glob enabled");

    test()
        .cwd(playground.path())
        .run("glob --dbg-matches '.'")
        .expect_value_eq(true)
        .expect("--dbg-matches should be available with dc-glob enabled");

    test()
        .cwd(playground.path())
        .run("glob --dbg-glob '*.txt' | length")
        .expect_value_eq(1)
        .expect("--dbg-glob should be available with dc-glob enabled");;

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn glob_debug_subcommands_disabled(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("glob --dbg-parse '*'")
        .expect_parse_error()
        .expect("glob should reject dbg flags when dc-glob is disabled");

    let err = err.to_string();
    assert!(
        err.contains("unknown_flag")
            || err.contains("unknown flag")
            || err.contains("doesn't have flag"),
        "expected unknown-flag parse error, got: {err}"
    );;

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_debug_subcommands_require_pattern_argument(playground: Playground) -> Result {
    let parse = test()
        .cwd(playground.path())
        .run("glob --dbg-parse")
        .expect_parse_error()
        .expect("--dbg-parse should require pattern argument");
    let parse = parse.to_string();
    assert!(
        parse.contains("missing")
            || parse.contains("required positional")
            || parse.contains("missing_positional"),
        "expected missing-argument parse error, got: {parse}"
    );

    let compile = test()
        .cwd(playground.path())
        .run("glob --dbg-compile")
        .expect_parse_error()
        .expect("--dbg-compile should require pattern argument");
    let compile = compile.to_string();
    assert!(
        compile.contains("missing")
            || compile.contains("required positional")
            || compile.contains("missing_positional"),
        "expected missing-argument parse error, got: {compile}"
    );

    let matches = test()
        .cwd(playground.path())
        .run("glob --dbg-matches")
        .expect_parse_error()
        .expect("--dbg-matches should require pattern argument");
    let matches = matches.to_string();
    assert!(
        matches.contains("missing")
            || matches.contains("required positional")
            || matches.contains("missing_positional"),
        "expected missing-argument parse error, got: {matches}"
    );

    let dbg_glob = test()
        .cwd(playground.path())
        .run("glob --dbg-glob")
        .expect_parse_error()
        .expect("--dbg-glob should require pattern argument");
    let dbg_glob = dbg_glob.to_string();
    assert!(
        dbg_glob.contains("missing")
            || dbg_glob.contains("required positional")
            || dbg_glob.contains("missing_positional"),
        "expected missing-argument parse error, got: {dbg_glob}"
    );;

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_literal_prefix_wildcard() -> Result {
    Playground::setup("glob_dc_literal_prefix_wildcard", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox.within("subdir").with_files(&[
            EmptyFile("nu_test1"),
            EmptyFile("nu_test2"),
            EmptyFile("other"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob 'subdir/nu*' | length")
            .expect_value_eq(2)
            .expect("glob 'subdir/nu*' should match both nu_test files with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_wildcard_then_literal() -> Result {
    Playground::setup("glob_dc_wildcard_literal", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox.within("subdir").with_files(&[
            EmptyFile("nu_test1"),
            EmptyFile("nu_test2"),
            EmptyFile("other"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob 'subdir/*nu*' | length")
            .expect_value_eq(2)
            .expect("glob 'subdir/*nu*' should match both nu_test files with dc-glob");
    });

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_literal_prefix_wildcard_absolute() -> Result {
    Playground::setup("glob_dc_literal_prefix_abs", |dirs, sandbox| {
        sandbox.mkdir("subdir");
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("nu_test.txt")]);

        let pattern = format!("{}/subdir/nu*", dirs.test().to_string_lossy());
        test()
            .cwd(dirs.test())
            .run(format!("glob '{pattern}' | length"))
            .expect_value_eq(1)
            .expect("absolute glob pattern with literal-then-wildcard should work with dc-glob");
    });

    Ok(())
}

