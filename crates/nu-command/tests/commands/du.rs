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
fn du_files_with_glob_metachars(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
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
        .run("du -m 1 | columns")
        .expect_value_eq(["path", "apparent", "physical"])?;

    test()
        .cwd("tests/fixtures/formats")
        .run("du -m 1 -l | columns")
        .expect_value_eq(["path", "apparent", "physical", "directories", "files"])
}

#[test]
fn du_wildcards(playground: Playground) -> Result {
    playground.empty_file(".a")?;

    // by default, wildcard don't match dot files.
    test()
        .cwd(playground.path())
        .run("du * | length")
        .expect_value_eq(0)?;

    // unless `-a` flag is provided.
    test()
        .cwd(playground.path())
        .run("du -a * | length")
        .expect_value_eq(1)
}
