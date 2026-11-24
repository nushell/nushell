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

// wax also lists the start directory for `**/*`; dc-glob doesn't copy that, so this test pins the
// legacy backend and `glob_subdirs_dc_glob` covers dc-glob.
#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn glob_subdirs(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;
    playground.empty_file("children/timothy.txt")?;
    playground.empty_file("children/tiffany.txt")?;
    playground.empty_file("children/trish.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '**/*' | length")
        .expect_value_eq(8)
}

/// `**/*` lists the six files and the `children` directory, but not the start directory.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_subdirs_dc_glob(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;
    playground.dir("children")?;
    playground.dir("children")?;
    playground.empty_file("children/timothy.txt")?;
    playground.empty_file("children/tiffany.txt")?;
    playground.empty_file("children/trish.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '**/*' | length")
        .expect_value_eq(7)
}

/// A drive-relative pattern like `C:*.txt` matches in `$env.PWD`, not the process's directory,
/// even when `$env.PWD` has glob characters like `[` in it.
#[cfg(windows)]
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_drive_relative_pattern_dc_glob(playground: Playground) -> Result {
    playground.dir("work [old]")?;
    playground.empty_file("work [old]/yehuda.txt")?;
    playground.empty_file("work [old]/jttxt")?;
    playground.dir("work [old]/sub")?;
    playground.empty_file("work [old]/sub/andres.txt")?;
    let pwd = playground.path().join("work [old]");

    // e.g. `C:`, the drive `$env.PWD` is on
    let drive = playground.path().components().next().unwrap();
    let drive = drive.as_os_str().to_string_lossy();

    test()
        .cwd(&pwd)
        .run(format!("glob {drive}*.txt | path basename"))
        .expect_value_eq(["yehuda.txt"])?;
    test()
        .cwd(&pwd)
        .run(format!("glob {drive}sub/*.txt | path basename"))
        .expect_value_eq(["andres.txt"])?;
    test()
        .cwd(&pwd)
        .run(format!("glob {drive} | path basename"))
        .expect_value_eq(["work [old]"])
}

#[test]
fn glob_subdirs_ignore_dirs(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;
    playground.empty_file("children/timothy.txt")?;
    playground.empty_file("children/tiffany.txt")?;
    playground.empty_file("children/trish.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '**/*' -D | length")
        .expect_value_eq(6)
}

#[test]
fn glob_ignore_files(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;
    playground.empty_file("children/timothy.txt")?;
    playground.empty_file("children/tiffany.txt")?;
    playground.empty_file("children/trish.txt")?;

    test()
        .cwd(playground.path())
        .run("glob '*' -F | length")
        .expect_value_eq(1)
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
#[case("foo/bar", "'../*'", &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/bar/../able", "foo/bar/../bar", "foo/bar/../charlie"], "glob matches files in parent")]
#[case("foo", "'./{a,b}*'", &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/able", "foo/bar"], "glob with leading ./ matches peer files")]
fn glob_files_in_parent(
    #[ignore] playground: Playground,
    #[case] wd: &str,
    #[case] glob: &str,
    #[case] ini: &[&str],
    #[case] exp: &[&str],
    #[case] tag: &str,
) -> Result {
    playground.dir("foo/bar")?;
    let working_directory = playground.path().join(wd);

    for f in ini {
        create_file_at(playground.path().join(f))?;
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

        expected.push(playground.path().join(e).to_string_lossy().to_string());
    }

    let expected = expected.join(" ");
    assert_eq!(actual, expected, "\n  test: {tag}");

    Ok(())
}

/// wax lets `\` escape a class member. With dc-glob, `glob` follows nu-glob, where `[]}]` does
/// that instead (see `glob_dc_glob_class_escape_and_dot_cwd`).
#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn glob_wax_class_escape(playground: Playground) -> Result {
    playground.dir("foo")?;
    playground.dir("foo/bar")?;
    playground.empty_file("foo/bar/ab}le")?;
    playground.empty_file("foo/bar/baker")?;
    playground.empty_file("foo/bar/cha]rlie")?;

    test()
        .cwd(playground.path().join("foo/bar"))
        .run(r"glob '*[\]}]*' | path basename | sort")
        .expect_value_eq(["ab}le", "cha]rlie"])
}

/// As with nu-glob, a `]` right after `[` is a class member, and a cwd ending in `.` doesn't put
/// `/./` in the matches.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_class_escape_and_dot_cwd(playground: Playground) -> Result {
    playground.dir("foo")?;
    playground.dir("foo/bar")?;
    playground.empty_file("foo/bar/ab}le")?;
    playground.empty_file("foo/bar/baker")?;
    playground.empty_file("foo/bar/cha]rlie")?;

    test()
        .cwd(playground.path().join("foo/bar"))
        .run("glob '*[]}]*' | path basename | sort")
        .expect_value_eq(["ab}le", "cha]rlie"])?;

    test()
        .cwd(playground.path().join("foo/bar/."))
        .run("glob 'b*' | each { str contains $'(char psep).(char psep)' }")
        .expect_value_eq([false])
}

#[test]
fn glob_follow_symlinks(playground: Playground) -> Result {
    playground.empty_file("target_dir/target_file.txt")?;
    playground.symlink("target_dir", "symlink_dir")?;

    // on some systems/filesystems, symlinks are followed by default
    // on others (like Linux /sys), they aren't
    // Test that with the --follow-symlinks flag, files are found for sure
    test()
        .cwd(playground.path())
        .run("glob 'symlink_dir/*.txt' --follow-symlinks | length")
        .expect_value_eq(1)
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_explicit_relative_paths(playground: Playground) -> Result {
    playground.empty_file("root.txt")?;
    playground.dir("a")?;
    playground.empty_file("a/parent.txt")?;
    playground.dir("a/b")?;
    playground.empty_file("a/b/local.txt")?;
    playground.dir("a/b/deep")?;
    playground.empty_file("a/b/deep/nested.txt")?;
    playground.dir("a/sibling")?;
    playground.empty_file("a/sibling/sibling.txt")?;

    for (pattern, expected) in [
        (".", "[$cwd]"),
        ("..", "[$parent]"),
        ("./*.txt", "[($cwd | path join local.txt)]"),
        ("../*.txt", "[($parent | path join parent.txt)]"),
        ("../../*.txt", "[($root | path join root.txt)]"),
        // As with nu-glob: the directories below the prefix.
        (
            "../**",
            "[$cwd ($cwd | path join deep) ($parent | path join sibling)]",
        ),
        (
            "../**/*.txt",
            "[($parent | path join parent.txt) ($cwd | path join local.txt) ($cwd | path join deep/nested.txt) ($parent | path join sibling/sibling.txt)]",
        ),
        // `...` is `../..`, as with the legacy backend.
        ("...", "[$root]"),
        (".../*.txt", "[($root | path join root.txt)]"),
        (
            ".../**",
            "[$parent $cwd ($cwd | path join deep) ($parent | path join sibling)]",
        ),
        ("./.../*.txt", "[($root | path join root.txt)]"),
    ] {
        test()
            .cwd(playground.path().join("a/b"))
            .run(format!(
                "let cwd = (pwd | path expand); \
                     let parent = ($cwd | path dirname); \
                     let root = ($parent | path dirname); \
                     let expected = ({expected} | each {{ path expand }} | sort); \
                     (glob '{pattern}' | each {{ path expand }} | sort) == $expected"
            ))
            .expect_value_eq(true)
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
    }

    Ok(())
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
        .expect("glob should support --depth and --exclude with dc-glob enabled");

    Ok(())
}

/// With dc-glob, `--exclude` takes `{a,b}` alternatives, as the pattern does.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_exclude_braces(playground: Playground) -> Result {
    playground.empty_file("a.rs")?;
    playground.dir("src")?;
    playground.empty_file("src/x.rs")?;
    playground.dir("tests")?;
    playground.empty_file("tests/y.rs")?;
    playground.dir("docs")?;
    playground.empty_file("docs/z.rs")?;

    test()
        .cwd(playground.path())
        .run("glob '**/*.rs' --exclude ['{src,tests}/**'] | path relative-to (pwd) | sort")
        .expect_value_eq([
            "a.rs".to_string(),
            format!("docs{}z.rs", std::path::MAIN_SEPARATOR),
        ])
}

/// With dc-glob, `--exclude` and `--dbg-glob` read `...` as `../..`, as the pattern does.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_ndots_in_exclude_and_dbg_glob(playground: Playground) -> Result {
    playground.empty_file("root.txt")?;
    playground.empty_file("skip/skipped.txt")?;
    playground.empty_file("a/b/local.txt")?;

    test()
        .cwd(playground.path().join("a/b"))
        .run("glob '.../**/*.txt' --exclude ['.../skip/*'] | path basename | sort")
        .expect_value_eq(["local.txt", "root.txt"])?;

    test()
        .cwd(playground.path().join("a/b"))
        .run("glob --dbg-glob '.../*.txt' | path basename")
        .expect_value_eq(["root.txt"])
}

/// Regression tests for https://github.com/nushell/nushell/issues/18600
///
/// With dc-glob, `glob` follows nu-glob's rules:
/// - bare `**` lists the directories below the start dir, not the start dir or files
/// - `**/*` lists files and dirs under the start, **not** the start itself
/// - extra `/*` segments enforce a minimum path depth
/// - `foo/**` lists the directories below `foo`
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_recursive_depth_semantics(playground: Playground) -> Result {
    playground.dir("1")?;
    playground.dir("1/2")?;
    playground.dir("1/2/3")?;
    playground.dir("1/2/3")?;
    playground.empty_file("1/2/3/file.txt")?;
    playground.dir("foo")?;
    playground.dir("foo/bar")?;
    playground.dir("foo")?;
    playground.empty_file("foo/sibling.txt")?;

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
        .cwd(playground.path())
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
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(true)
        .expect("**/*/*/* should only match depth >= 3 paths");

    // bare ** → the directories below the start: 1, 1/2, 1/2/3, foo, foo/bar
    let code = "
            let root = (pwd | path expand)
            let paths = (glob '**' | each { path expand } | sort)
            (
                ($paths | length) == 5
                and not ($paths | any {|p| $p == $root })
                and ($paths | any {|p| $p | str ends-with $'(char psep)1'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)1(char psep)2(char psep)3'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo'})
                and ($paths | any {|p| $p | str ends-with $'(char psep)foo(char psep)bar'})
            )
        ";
    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(true)
        .expect("bare ** should list the directories below the start");

    // foo/** lists the directories below foo, not foo or its files
    test()
        .cwd(playground.path())
        .run("glob 'foo/**' | path relative-to (pwd)")
        .expect_value_eq([format!("foo{}bar", std::path::MAIN_SEPARATOR)])
        .expect("foo/** should list the directories below foo");

    // `*/**` lists the directories below `1` and `foo`, not them: `*` has to match a whole
    // component.
    test()
        .cwd(playground.path())
        .run("glob '*/**' | path relative-to (pwd) | sort")
        .expect_value_eq([
            format!("1{}2", std::path::MAIN_SEPARATOR),
            format!("1{0}2{0}3", std::path::MAIN_SEPARATOR),
            format!("foo{}bar", std::path::MAIN_SEPARATOR),
        ])
        .expect("*/** should list the directories below the top-level ones");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_supports_follow_symlinks(playground: Playground) -> Result {
    playground.empty_file("target_dir/target_file.txt")?;
    playground.symlink("target_dir", "symlink_dir")?;

    test()
        .cwd(playground.path())
        .run("glob 'symlink_dir/*.txt' --follow-symlinks | length")
        .expect_value_eq(1)
        .expect("glob should follow symlinked dirs when --follow-symlinks is set");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_follows_aliases_without_descending_into_cycles(playground: Playground) -> Result {
    playground.empty_file("target/marker.txt")?;
    playground.symlink("target", "alias")?;
    playground.symlink("target", "target/up")?;

    let mut tester = test().cwd(playground.path());
    tester.run("glob '**/*.txt' | length").expect_value_eq(1)?;
    tester
        .run("glob '**/*.txt' --follow-symlinks | first 3 | each { path relative-to (pwd) | path split | str join '/' } | sort")
        .expect_value_eq(["alias/marker.txt", "target/marker.txt"])?;
    tester
        .run("glob '*/up/*.txt' --follow-symlinks | length")
        .expect_value_eq(2)
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_ignore_case_resolves_symlink_prefix_before_tracking_cycles(
    playground: Playground,
) -> Result {
    playground.empty_file("marker.txt")?;
    playground.empty_file("nested/other.txt")?;
    playground.symlink(".", "alias")?;
    let mut tester = test().cwd(playground.path());

    tester
        .run("glob -i -d 1 'ALIAS/*.txt' --follow-symlinks | path basename")
        .expect_value_eq(["marker.txt"])?;
    tester
        .run("glob -i 'ALIAS/**' --follow-symlinks | first 3 | each { path relative-to (pwd) | path split | str join '/' } | sort")
        .expect_value_eq(["alias/alias", "alias/nested"])
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
        .expect("--dbg-glob should be available with dc-glob enabled");

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
    );

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
    );

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_literal_prefix_wildcard(playground: Playground) -> Result {
    playground.empty_file("subdir/nu_test1")?;
    playground.empty_file("subdir/nu_test2")?;
    playground.empty_file("subdir/other")?;

    test()
        .cwd(playground.path())
        .run("glob 'subdir/nu*' | length")
        .expect_value_eq(2)
        .expect("glob 'subdir/nu*' should match both nu_test files with dc-glob");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_wildcard_then_literal(playground: Playground) -> Result {
    playground.empty_file("subdir/nu_test1")?;
    playground.empty_file("subdir/nu_test2")?;
    playground.empty_file("subdir/other")?;

    test()
        .cwd(playground.path())
        .run("glob 'subdir/*nu*' | length")
        .expect_value_eq(2)
        .expect("glob 'subdir/*nu*' should match both nu_test files with dc-glob");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_literal_prefix_wildcard_absolute(playground: Playground) -> Result {
    playground.empty_file("subdir/nu_test.txt")?;

    let pattern = format!("{}/subdir/nu*", playground.path().to_string_lossy());
    test()
        .cwd(playground.path())
        .run(format!("glob '{pattern}' | length"))
        .expect_value_eq(1)
        .expect("absolute glob pattern with literal-then-wildcard should work with dc-glob");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_ignore_case_depth_counts_from_the_same_place(playground: Playground) -> Result {
    std::fs::create_dir_all(playground.path().join("docs")).expect("can not create docs");
    playground.file("docs/note.md", "")?;

    // With `-i` the traversal starts at the base directory and leaves the
    // literal `docs` to the matcher, so `--depth` has to count from the
    // same place either way. A `..` after the literal is the exception:
    // the parent has to be resolved by the filesystem, so the literal goes
    // into the traversal start directory together with it.
    for pattern in [
        "glob 'docs/*.md' | length",
        "glob -i 'docs/*.md' | length",
        "glob 'docs/../docs/*.md' | length",
        "glob -i 'docs/../docs/*.md' | length",
    ] {
        test()
            .cwd(playground.path())
            .run(pattern)
            .expect_value_eq(1)
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
    }

    for pattern in [
        "glob -d 1 'docs/*.md' | length",
        "glob -i -d 1 'docs/*.md' | length",
        "glob -d 1 'docs/../docs/*.md' | length",
        "glob -i -d 1 'docs/../docs/*.md' | length",
    ] {
        test()
            .cwd(playground.path())
            .run(pattern)
            .expect_value_eq(1)
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
    }

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_ignore_case_keeps_parent_components(playground: Playground) -> Result {
    std::fs::create_dir_all(playground.path().join("docs/deep")).expect("can not create docs/deep");
    playground.file("docs/note.md", "")?;
    playground.file("docs/deep/x.md", "")?;

    // A `..` after a literal component has to be resolved by the
    // filesystem. With `-i` the literal used to be left to the matcher,
    // so the traversal started above the `..` and never reached the
    // parent: the pattern matched nothing.
    for pattern in [
        "glob 'docs/../docs/*.md' | length",
        "glob -i 'docs/../docs/*.md' | length",
        "glob 'docs/deep/../*.md' | length",
        "glob -i 'docs/deep/../*.md' | length",
    ] {
        test()
            .cwd(playground.path())
            .run(pattern)
            .expect_value_eq(1)
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
    }

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_current_directory_has_no_trailing_separator(playground: Playground) -> Result {
    playground.empty_file("root.txt")?;

    test()
        .cwd(playground.path())
        .run("(glob '.' | first) == (pwd)")
        .expect_value_eq(true)
        .expect("`glob .` should return the current directory without a trailing separator");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_ignore_case_matches_literal_file(playground: Playground) -> Result {
    playground.empty_file("README.md")?;

    test()
        .cwd(playground.path())
        .run("glob -i readme.md | path basename | first")
        .expect_value_eq("README.md")
        .expect("`glob -i` should match a literal path that differs only in case");

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_dc_glob_missing_path_returns_empty_list(playground: Playground) -> Result {
    playground.empty_file("root.txt")?;

    for pattern in ["missing/x.txt", "missing/*.txt"] {
        test()
            .cwd(playground.path())
            .run(format!("glob '{pattern}' | length"))
            .expect_value_eq(0)
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
    }

    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn glob_matches_hidden_names_legacy_glob(playground: Playground) -> Result {
    check_glob_matches_hidden_names(&playground)
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn glob_matches_hidden_names_dc_glob(playground: Playground) -> Result {
    check_glob_matches_hidden_names(&playground)
}

/// `glob` has no hidden-name rule: `*` and `**` match names that start with `.`, on both glob
/// backends. It's the way to list every path, hidden ones included, where `rm`, `cp`, `mv` and
/// `du` skip them without `--all`.
fn check_glob_matches_hidden_names(playground: &Playground) -> Result {
    playground.empty_file(".top")?;
    playground.empty_file("x")?;
    playground.dir(".e")?;
    playground.empty_file(".e/y")?;

    let names = |pattern: &str| -> Result<Vec<String>> {
        test().cwd(playground.path()).run(format!(
            "glob '{pattern}' | each {{ path basename }} | sort"
        ))
    };
    assert_eq!(names("*")?, [".e", ".top", "x"]);
    assert_eq!(names("**/y")?, ["y"]);
    Ok(())
}
