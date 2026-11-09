//! The winnow syntax tree the shell's front end reads for each fixture of nu-parser's winnow
//! tests (`crates/nu-parser/tests/winnow/fixtures`), with the test engine's commands and the
//! `winnow-parser` option on: each statement of the file as the driver hands it to the lowering,
//! with its diagnostics (`nu_parser::winnow::capture_statement_trees`).
//!
//! nu-parser's own fixture tests pin the tree the winnow parser builds alone, with the commands
//! it knows from its configuration. In the shell it asks the engine instead, so the tree can
//! differ; these goldens, `<name>.shell` next to each fixture, pin that one. Regenerate them after
//! an intentional change with
//!
//! ```text
//! UPDATE_FIXTURES=1 cargo test --test tests -- parsing::winnow_shell_trees
//! ```
//!
//! and review the diff.

use std::path::{Path, PathBuf};

use nu_protocol::engine::StateWorkingSet;
use nu_test_support::prelude::*;
use rstest::rstest;

/// Compare `actual` with the golden file at `golden`, writing it when `UPDATE_FIXTURES` is set.
fn check_golden(golden: &Path, actual: &str) {
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(golden, actual)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", golden.display()));
        return;
    }
    let expected = std::fs::read_to_string(golden).unwrap_or_else(|_| {
        panic!(
            "missing golden file {}; run `UPDATE_FIXTURES=1 cargo test --test tests -- parsing::winnow_shell_trees` and review it",
            golden.display()
        )
    });
    assert_eq!(
        expected,
        actual,
        "{} differs from the tree the shell reads; run `UPDATE_FIXTURES=1 cargo test --test tests -- parsing::winnow_shell_trees` if the change is intended",
        golden.display()
    );
}

#[rstest]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn shell_tree(#[files("crates/nu-parser/tests/winnow/fixtures/**/*.nu")] path: PathBuf) -> Result {
    let source = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let (_, trees) = nu_parser::winnow::capture_statement_trees(|| {
        nu_parser::parse(&mut working_set, Some("fixture.nu"), &source, false)
    });
    check_golden(&path.with_extension("shell"), &trees.concat());
    Ok(())
}
