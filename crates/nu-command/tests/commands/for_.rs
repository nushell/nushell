use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn for_doesnt_auto_print_in_each_iteration() -> Result {
    let code = "
        for i in 1..2 {
            $i
        }
    ";

    test().run(code).expect_value_eq(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn for_break_on_external_failed() -> Result {
    let code = "
        for i in 1..2 {
            fail
        }
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn failed_for_should_break_running() -> Result {
    let code = "
        for i in 1..2 {
            fail
        }
        3
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")?;

    let code = "
        let x = [1 2]
        for i in $x {
            fail
        }
        3
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
fn for_loops_dont_collect_source() -> Result {
    Playground::setup("for_loops_dont_collect_source", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("out.txt")]);

        let code = "
            for i in (seq 1 10 | each { $in | save --append out.txt; $in }) {
                $i | save --append out.txt
                if $i >= 5 { break }
            }
            open --raw out.txt
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("1122334455")
    })
}

// Regression test for https://github.com/nushell/nushell/issues/13746
// Passing a non-block (e.g. the loop variable) as the `for` block used to panic
// on the `.expect("internal error: missing block")` in `for`'s `run`. It must
// surface a clean error instead and never panic.
#[rstest]
#[case("for i in [1 2 3] $i")]
#[case("for i in [] $i")]
fn for_with_non_block_body_errors_without_panic(#[case] src: &str) -> Result {
    let err = test().run(src).expect_compile_error()?;
    assert_matches!(err, CompileError::InvalidKeywordCall { .. });
    Ok(())
}
