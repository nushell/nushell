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

// A file reached through several hard links is one file, and `du` counts it once
// unless --count-links says otherwise. See GNU `du --count-links`, BSD `du -l`,
// and POSIX: "A file that occurs multiple times shall be counted and written for
// only one entry, even if the occurrences are under different file operands."
#[test]
#[cfg_attr(windows, ignore = "creating hard links can require privileges")]
fn du_counts_a_hard_linked_file_once() -> Result {
    Playground::setup("du_hard_links", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("a", "0123456789")]);
        let dir = dirs.test();
        std::fs::hard_link(dir.join("a"), dir.join("b")).expect("failed to create a hard link");

        // Comparing the two runs cancels out the size of the directory entry
        // itself, which `du` also counts and which varies between filesystems.
        // The difference is exactly the one file counted a second time.
        test()
            .cwd(dir)
            .run("((du --count-links . | get 0.apparent) - (du . | get 0.apparent)) | into int")
            .expect_value_eq(10)
    })
}

// A file with only one link can still be reached twice when operands overlap,
// so every file is recorded, not just the hard-linked ones. GNU `du` switches
// its own link-count shortcut off for exactly this case.
#[test]
fn du_counts_a_file_named_twice_only_once() -> Result {
    Playground::setup("du_repeated_operand", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("a", "0123456789")]);
        let dir = dirs.test();

        test().cwd(dir).run("du a a | length").expect_value_eq(1)?;
        test()
            .cwd(dir)
            .run("du a a | get apparent | math sum | into int")
            .expect_value_eq(10)?;

        // --count-links restores one entry per occurrence.
        test()
            .cwd(dir)
            .run("du --count-links a a | length")
            .expect_value_eq(2)
    })
}

// The set of device-inode pairs spans the whole command rather than one path
// operand, so naming both links on one command line still counts and writes the
// file only once.
#[test]
#[cfg_attr(windows, ignore = "creating hard links can require privileges")]
fn du_counts_a_hard_linked_file_once_across_operands() -> Result {
    Playground::setup("du_hard_links_operands", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("a", "0123456789")]);
        let dir = dirs.test();
        std::fs::hard_link(dir.join("a"), dir.join("b")).expect("failed to create a hard link");

        test().cwd(dir).run("du a b | length").expect_value_eq(1)?;
        test()
            .cwd(dir)
            .run("du a b | get apparent | math sum | into int")
            .expect_value_eq(10)?;

        // --count-links restores one entry per link.
        test()
            .cwd(dir)
            .run("du --count-links a b | length")
            .expect_value_eq(2)?;
        test()
            .cwd(dir)
            .run("du --count-links a b | get apparent | math sum | into int")
            .expect_value_eq(20)
    })
}
