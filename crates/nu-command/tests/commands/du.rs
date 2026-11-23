use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn test_du_flag_min_size() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("du -m -1")
        .expect_error_code_eq("nu::shell::needs_positive_value")?;

    let _: Value = test().cwd("tests/fixtures/formats").run("du -m 1")?;
    Ok(())
}

/// `scope commands` reports `--min-size` as deprecated.
#[test]
fn test_du_flag_min_size_deprecated() -> Result {
    test()
        .run("scope commands | where name == du | get 0.deprecation_info.flag")
        .expect_value_eq(["min-size"])
}

#[test]
fn test_du_flag_max_depth() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("du -d -2")
        .expect_error_code_eq("nu::shell::needs_positive_value")?;

    let _: Value = test().cwd("tests/fixtures/formats").run("du -d 2")?;
    Ok(())
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg_attr(windows, ignore = "invalid path")]
#[case("a]?c")]
#[cfg_attr(windows, ignore = "invalid path")]
#[case("a*.?c")]
fn du_files_with_glob_metachars(
    #[case] src_name: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.empty_file(src_name)?;

    let src = playground.path().join(src_name);
    let code = format!("du -d 1 '{}'", src.display());
    let _: Value = test().cwd(playground.path()).run(code)?;

    let code = format!("let f = '{}'; du -d 1 $f", src.display());
    let _: Value = test().cwd(playground.path()).run(code)?;
    Ok(())
}

#[test]
fn du_with_multiple_path() -> Result {
    let paths: Vec<String> = test()
        .cwd("tests/fixtures")
        .run("du cp formats | get path | path basename")?;

    assert!(paths.iter().any(|path| path == "cp"));
    assert!(paths.iter().any(|path| path == "formats"));
    assert!(!paths.iter().any(|path| path == "lsp"));

    // report errors if one path not exists
    test()
        .cwd("tests/fixtures")
        .run("du cp asdf | get path | path basename")
        .expect_error_code_eq("nu::shell::io::not_found")?;

    // du with spreading empty list should returns nothing.
    test()
        .cwd("tests/fixtures")
        .run("du ...[] | length")
        .expect_value_eq(0)
}

#[test]
fn test_du_output_columns() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("du | columns")
        .expect_value_eq(["path", "apparent", "physical"])?;

    test()
        .cwd("tests/fixtures/formats")
        .run("du -l | columns")
        .expect_value_eq(["path", "apparent", "physical", "directories", "files"])
}

/// `--exclude` matches each file's full path, and `*` also matches `/`, so `*.txt` leaves out the
/// `.txt` files at every depth, with dc-glob on and off.
fn du_exclude_matches_at_any_depth(playground: &Playground) -> Result {
    playground.dir("data")?;
    playground.empty_file("data/a.txt")?;
    playground.empty_file("data/b.rs")?;
    playground.dir("data/sub")?;
    playground.empty_file("data/sub/c.txt")?;
    playground.empty_file("data/sub/d.toml")?;

    test()
        .cwd(playground.path())
        .run(
            "let data = (du data -l --exclude '*.txt' | get 0); \
                 $data.files.path ++ $data.directories.0.files.path | path basename | sort",
        )
        .expect_value_eq(["b.rs", "d.toml"])
}

#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn du_exclude_matches_at_any_depth_legacy_glob(playground: Playground) -> Result {
    du_exclude_matches_at_any_depth(&playground)
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn du_exclude_matches_at_any_depth_dc_glob(playground: Playground) -> Result {
    du_exclude_matches_at_any_depth(&playground)
}

/// With dc-glob, `--exclude` takes `{a,b}` alternatives.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn du_dc_glob_exclude_braces(playground: Playground) -> Result {
    playground.dir("data")?;
    playground.empty_file("data/a.txt")?;
    playground.empty_file("data/b.rs")?;
    playground.empty_file("data/c.toml")?;

    test()
        .cwd(playground.path())
        .run("du data -l --exclude '*.{txt,toml}' | get 0.files.path | path basename")
        .expect_value_eq(["b.rs"])
}

#[nu_test_support::test]
#[exp(nu_experimental::DC_GLOB = false)]
fn du_wildcards_legacy_glob(playground: Playground) -> Result {
    check_du_wildcards(&playground)
}

#[nu_test_support::test]
#[exp(nu_experimental::DC_GLOB)]
fn du_wildcards_dc_glob(playground: Playground) -> Result {
    check_du_wildcards(&playground)
}

/// A `*` or `**` given to `du` doesn't match a hidden name, at any depth, unless `-a` is given.
fn check_du_wildcards(playground: &Playground) -> Result {
    playground.empty_file(".a")?;
    playground.dir("d")?;
    playground.empty_file("d/x")?;
    playground.dir(".e")?;
    playground.empty_file(".e/y")?;

    let names = |command: &str| -> Result<Vec<String>> {
        test()
            .cwd(playground.path())
            .run(format!("{command} | get path | path basename | sort"))
    };
    assert_eq!(names("du *")?, ["d"]);
    assert_eq!(names("du **/*")?, ["d", "x"]);
    assert_eq!(names("du -a *")?, [".a", ".e", "d"]);
    Ok(())
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn du_counts_hard_links_once(playground: Playground) -> Result {
    // 1 MiB of real (allocated) bytes; the directory entry overhead stays
    // orders of magnitude below the assertion bound either way.
    let body = "x".repeat(1024 * 1024);
    playground.dir("links")?;
    playground.file("links/a", &body)?;
    drop(body);

    let links = playground.path().join("links");
    std::fs::hard_link(links.join("a"), links.join("b")).expect("link b");
    std::fs::hard_link(links.join("a"), links.join("c")).expect("link c");

    // `a`, `b` and `c` are the same file: both columns must report ~1 MiB,
    // not ~3 MiB. The lower bound catches a walk that counted nothing.
    let sizes: Vec<i64> = test()
        .cwd(playground.path())
        .run("du links | first | select apparent physical | values | into int")?;

    assert!(
        sizes[0] >= 1024 * 1024,
        "hard-linked file not counted: apparent = {}",
        sizes[0]
    );
    assert!(
        sizes[0] < 2 * 1024 * 1024,
        "hard-linked file counted more than once: apparent = {}",
        sizes[0]
    );
    // No lower bound for `physical`: file systems with transparent
    // compression report less than the allocated bytes.
    assert!(
        sizes[1] < 2 * 1024 * 1024,
        "hard-linked file counted more than once: physical = {}",
        sizes[1]
    );
    Ok(())
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn du_counts_hard_links_once_across_paths(playground: Playground) -> Result {
    let body = "x".repeat(1024 * 1024);
    playground.dir("d1")?;
    playground.file("d1/a", &body)?;
    drop(body);
    playground.dir("d2")?;

    std::fs::hard_link(
        playground.path().join("d1").join("a"),
        playground.path().join("d2").join("b"),
    )
    .expect("link b");

    // The link lives under both operands, so only the first row carries its
    // size; the second one is directory entry overhead.
    let sizes: Vec<i64> = test()
        .cwd(playground.path())
        .run("du d1 d2 | get apparent | into int")?;

    assert!(
        sizes.len() == 2,
        "expected one row per path operand, got {sizes:?}"
    );
    assert!(
        sizes[0] >= 1024 * 1024 && sizes[0] < 2 * 1024 * 1024,
        "hard-linked file not counted exactly once: apparent = {}",
        sizes[0]
    );
    assert!(
        sizes[1] <= 4096,
        "hard-linked file counted under the second path too: apparent = {}",
        sizes[1]
    );
    Ok(())
}
