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

#[test]
fn du_wildcards() -> Result {
    Playground::setup("du_wildcards", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(".a")]);

        // by default, wildcard don't match dot files.
        test()
            .cwd(dirs.test())
            .run("du * | length")
            .expect_value_eq(0)?;

        // unless `-a` flag is provided.
        test()
            .cwd(dirs.test())
            .run("du -a * | length")
            .expect_value_eq(1)
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
