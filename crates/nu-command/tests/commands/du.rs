use nu_test_support::fs::Stub::{EmptyFile, FileWithContent};
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
#[cfg(unix)]
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
        // not ~3 MiB.
        let apparent: i64 = test()
            .cwd(dirs.test())
            .run("du links | first | get apparent | into int")?;
        let physical: i64 = test()
            .cwd(dirs.test())
            .run("du links | first | get physical | into int")?;

        assert!(
            apparent < 2 * 1024 * 1024,
            "hard-linked file counted more than once: apparent = {apparent}"
        );
        assert!(
            physical < 2 * 1024 * 1024,
            "hard-linked file counted more than once: physical = {physical}"
        );
        Ok(())
    })
}
