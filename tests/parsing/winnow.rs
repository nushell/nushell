//! The `winnow-parser` front end must parse what the classic parser parses, the same way.
//!
//! Most of that is checked by comparing both front ends over whole corpora (the `frontends`
//! harness in `crates/nu-winnow-parser/tools/nushell-harness`). The tests here pin cases the
//! corpora do not reach. The ones about parsing ahead need a long block (`AHEAD_MIN_BYTES` in
//! `crates/nu-parser/src/winnow/driver.rs`) and a second thread: on a machine that runs one
//! thread at a time they pass without exercising it.

use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
};
use nu_test_support::{fs::Stub, playground::Playground, prelude::*};
use rstest::rstest;

/// `count` statements that each build a closure, about 45 bytes apiece: enough of them make a
/// block long enough to be parsed ahead.
fn filler(count: usize) -> String {
    (1..=count)
        .map(|i| format!("    let x{i} = [1 2 3] | each {{|it| $it + {i} }}\n"))
        .collect()
}

/// When the thread parsing ahead resolved a name differently from the live working set (here
/// it takes the imported `def --wrapped` for an ordinary command), the classic parser takes the
/// rest of the block from that statement. In a `{ ... }` block it parses up to the closing
/// brace, not through it.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn fallback_inside_a_long_block_stops_at_its_brace() -> Result {
    Playground::setup("winnow_fallback_inside_a_long_block", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "w.nu",
            "export def --wrapped wr [...rest] { $rest | length }",
        )]);
        let code = format!(
            "do {{\n    use w.nu wr\n    let a = (wr --foo bar)\n{}    $a\n}}",
            filler(100)
        );
        test().cwd(dirs.test()).run(code).expect_value_eq(2)
    })
}

/// A block whose own `def` has a longer name than any command declared before is still parsed
/// ahead: the thread parsing ahead knows the block's definitions.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn long_definition_name_keeps_parsing_ahead() -> Result {
    let name = "a".repeat(300);
    let code = format!("def {name} [] {{ 1 }}\n{}{name}", filler(100));
    let before = nu_parser::winnow_stats();
    test().run(code).expect_value_eq(1)?;
    let after = nu_parser::winnow_stats();
    assert_eq!(after.ahead_fallbacks, before.ahead_fallbacks);
    if std::thread::available_parallelism().is_ok_and(|cpus| cpus.get() > 1) {
        assert!(
            after.ahead_runs > before.ahead_runs,
            "the block was not parsed ahead"
        );
    }
    Ok(())
}

/// In a glob position, `$"..."` is a glob only when it has a `(` (a subexpression to
/// interpolate), as in the classic parser's `parse_dollar_expr`; otherwise it is a string.
#[rstest]
#[case::without_subexpression(r#"ls $"foo""#, false)]
#[case::with_subexpression(r#"ls $"(1)""#, true)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn dollar_string_in_a_glob_position(#[case] code: &str, #[case] glob: bool) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}` is not a call");
    };
    let argument: Option<&Expression> = call.positional_iter().next();
    let expected = if glob {
        "GlobInterpolation"
    } else {
        "StringInterpolation"
    };
    match argument.map(|argument| &argument.expr) {
        Some(Expr::GlobInterpolation(..)) if glob => Ok(()),
        Some(Expr::StringInterpolation(..)) if !glob => Ok(()),
        other => panic!("`{code}`: expected {expected}, got {other:?}"),
    }
}
