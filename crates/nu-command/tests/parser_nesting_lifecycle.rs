//! Report-once state for `ParseError::NestingTooDeep` when `source` re-enters the parser through
//! `parse_fresh` mid-parse Lives here rather than in `nu-parser` because a real `source` needs
//! the shell command context

use nu_protocol::{ParseError, Value, engine::StateWorkingSet};
use std::sync::atomic::{AtomicUsize, Ordering};

fn deep_parens() -> String {
    format!("{}1{}", "(".repeat(200), ")".repeat(200))
}

fn deep_lists() -> String {
    format!("{}{}", "[".repeat(200), "]".repeat(200))
}

/// Parses `parent` with `child.nu` beside it returning the opening delimiter of each nesting
/// diagnostic `[` from the list-nested source `(` from the paren-nested one Runs on a worker
/// with a production-sized parser stack since reaching the nesting limit costs more recursion
/// than a default test thread holds in a debug build
fn nesting_diagnostic_delimiters(parent: &str, child: &str) -> Vec<char> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "nu_nesting_lifecycle_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("child.nu"), child).expect("write child");

    let parent = parent.to_string();
    let scan_dir = dir.clone();
    let delimiters = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let engine_state = nu_cmd_lang::create_default_context();
            let mut engine_state = nu_command::add_shell_command_context(engine_state);
            engine_state.add_env_var("PWD".into(), Value::test_string(scan_dir.to_string_lossy()));

            let mut working_set = StateWorkingSet::new(&engine_state);
            nu_parser::parse(
                &mut working_set,
                Some("parent.nu"),
                parent.as_bytes(),
                false,
            );

            working_set
                .parse_errors
                .iter()
                .filter_map(|err| match err {
                    ParseError::NestingTooDeep(span) => working_set
                        .get_span_contents(*span)
                        .first()
                        .map(|byte| *byte as char),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .expect("spawn parser worker")
        .join()
        .expect("parser worker finished");

    let _ = std::fs::remove_dir_all(&dir);
    delimiters
}

/// The parent reports before sourcing the child and must not report a second time afterwards
#[test]
fn nested_source_keeps_parent_report_state() {
    let parent = format!("{}\nsource child.nu\n{}", deep_lists(), deep_lists());
    let delimiters = nesting_diagnostic_delimiters(&parent, &deep_parens());

    assert_eq!(
        delimiters,
        vec!['[', '('],
        "expected one diagnostic for the parent and one for the child"
    );
}

#[test]
fn nested_source_does_not_consume_parent_report_state() {
    let parent = format!("source child.nu\n{}", deep_lists());
    let delimiters = nesting_diagnostic_delimiters(&parent, &deep_parens());

    assert_eq!(
        delimiters,
        vec!['(', '['],
        "child reported first, then the parent reported its own"
    );
}

/// A shallow child must not clear the parents report state and re-enable a second diagnostic
#[test]
fn shallow_child_does_not_reset_parent_report_state() {
    let parent = format!("{}\nsource child.nu\n{}", deep_lists(), deep_lists());
    let delimiters = nesting_diagnostic_delimiters(&parent, "[1 2 3] | length");

    assert_eq!(
        delimiters,
        vec!['['],
        "one diagnostic for the parent, none added by the shallow child"
    );
}
