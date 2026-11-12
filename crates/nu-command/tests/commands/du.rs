use nu_test_support::fs::Stub::EmptyFile;
#[cfg(not(target_arch = "wasm32"))]
use nu_test_support::fs::Stub::FileWithContent;
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
fn du_files_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("du_test_16", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(src_name)]);

        let src = dirs.test().join(src_name);
        let code = format!("du -d 1 '{}'", src.display());
        let _: Value = test().cwd(dirs.test()).run(code)?;

        let code = format!("let f = '{}'; du -d 1 $f", src.display());
        let _: Value = test().cwd(dirs.test()).run(code)?;
        Ok(())
    })
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
        .run("du -m 1 | columns")
        .expect_value_eq(["path", "apparent", "physical"])?;

    test()
        .cwd("tests/fixtures/formats")
        .run("du -m 1 -l | columns")
        .expect_value_eq(["path", "apparent", "physical", "directories", "files"])
}

/// `--exclude` matches each file's full path, and `*` also matches `/`, so `*.txt` leaves out the
/// `.txt` files at every depth, with dc-glob on and off.
fn du_exclude_matches_at_any_depth() -> Result {
    Playground::setup("du_exclude_any_depth", |dirs, sandbox| {
        sandbox
            .within("data")
            .with_files(&[EmptyFile("a.txt"), EmptyFile("b.rs")]);
        sandbox
            .within("data/sub")
            .with_files(&[EmptyFile("c.txt"), EmptyFile("d.toml")]);

        test()
            .cwd(dirs.test())
            .run(
                "let data = (du data -l --exclude '*.txt' | get 0); \
                 $data.files.path ++ $data.directories.0.files.path | path basename | sort",
            )
            .expect_value_eq(["b.rs", "d.toml"])
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB = false)]
fn du_exclude_matches_at_any_depth_legacy_glob() -> Result {
    du_exclude_matches_at_any_depth()
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn du_exclude_matches_at_any_depth_dc_glob() -> Result {
    du_exclude_matches_at_any_depth()
}

/// With dc-glob, `--exclude` takes `{a,b}` alternatives.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn du_dc_glob_exclude_braces() -> Result {
    Playground::setup("du_exclude_braces", |dirs, sandbox| {
        sandbox.within("data").with_files(&[
            EmptyFile("a.txt"),
            EmptyFile("b.rs"),
            EmptyFile("c.toml"),
        ]);

        test()
            .cwd(dirs.test())
            .run("du data -l --exclude '*.{txt,toml}' | get 0.files.path | path basename")
            .expect_value_eq(["b.rs"])
    })
}

#[nu_test_support::test]
#[exp(nu_experimental::DC_GLOB = false)]
fn du_wildcards_legacy_glob() -> Result {
    check_du_wildcards()
}

#[nu_test_support::test]
#[exp(nu_experimental::DC_GLOB)]
fn du_wildcards_dc_glob() -> Result {
    check_du_wildcards()
}

/// A `*` or `**` given to `du` doesn't match a hidden name, at any depth, unless `-a` is given.
fn check_du_wildcards() -> Result {
    Playground::setup("du_wildcards", |dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile(".a")])
            .within("d")
            .with_files(&[EmptyFile("x")])
            .back_to_playground()
            .within(".e")
            .with_files(&[EmptyFile("y")]);

        let names = |command: &str| -> Result<Vec<String>> {
            test()
                .cwd(dirs.test())
                .run(format!("{command} | get path | path basename | sort"))
        };
        assert_eq!(names("du *")?, ["d"]);
        assert_eq!(names("du **/*")?, ["d", "x"]);
        assert_eq!(names("du -a *")?, [".a", ".e", "d"]);
        Ok(())
    })
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn du_counts_hard_links_once() -> Result {
    Playground::setup("du_hard_links", |dirs, sandbox| {
        // 1 MiB of real (allocated) bytes; the directory entry overhead stays
        // orders of magnitude below the assertion bound either way.
        let body = "x".repeat(1024 * 1024);
        sandbox
            .within("links")
            .with_files(&[FileWithContent("a", &body)]);
        drop(body);

        let links = dirs.test().join("links");
        std::fs::hard_link(links.join("a"), links.join("b")).expect("link b");
        std::fs::hard_link(links.join("a"), links.join("c")).expect("link c");

        // `a`, `b` and `c` are the same file: both columns must report ~1 MiB,
        // not ~3 MiB. The lower bound catches a walk that counted nothing.
        let sizes: Vec<i64> = test()
            .cwd(dirs.test())
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
    })
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn du_counts_hard_links_once_across_paths() -> Result {
    Playground::setup("du_hard_links_across_paths", |dirs, sandbox| {
        let body = "x".repeat(1024 * 1024);
        sandbox
            .within("d1")
            .with_files(&[FileWithContent("a", &body)]);
        drop(body);
        sandbox.mkdir("d2");

        std::fs::hard_link(
            dirs.test().join("d1").join("a"),
            dirs.test().join("d2").join("b"),
        )
        .expect("link b");

        // The link lives under both operands, so only the first row carries its
        // size; the second one is directory entry overhead.
        let sizes: Vec<i64> = test()
            .cwd(dirs.test())
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
    })
}
