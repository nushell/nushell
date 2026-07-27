//! Interactive last-result (`$last`) capture tests.
//!
//! Last-result is only stored when `engine_state.is_interactive` is true and evaluation
//! goes through `eval_source` (the REPL path). `NuTester` uses `eval_block` without that
//! hook, so these tests drive `eval_source` directly while following harness conventions:
//! `Result` returns, structured value asserts, and no `nu!`.

use nu_cli::eval_source;
use nu_protocol::{
    Filesize, LAST_RESULT_VAR_NAME, LAST_VARIABLE_ID, PipelineData, Span, Value,
    engine::{EngineState, Stack},
};
use nu_test_support::prelude::*;
use std::sync::Arc;

/// Interactive engine + stack for last-result capture (REPL-equivalent path).
struct Interactive {
    engine_state: EngineState,
    stack: Stack,
}

impl Interactive {
    fn new() -> Self {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        engine_state.is_interactive = true;
        seed_pwd(&mut engine_state);
        Self {
            engine_state,
            stack: Stack::new(),
        }
    }

    fn non_interactive() -> Self {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        engine_state.is_interactive = false;
        seed_pwd(&mut engine_state);
        Self {
            engine_state,
            stack: Stack::new(),
        }
    }

    fn with_last_result_size(mut self, size: Filesize) -> Self {
        let mut cfg = self.engine_state.get_config().as_ref().clone();
        cfg.last_result_size = size;
        self.engine_state.config = Arc::new(cfg);
        self
    }

    /// One interactive source unit (like one REPL entry).
    fn run(&mut self, code: &str) {
        let _ = eval_source(
            &mut self.engine_state,
            &mut self.stack,
            code.as_bytes(),
            "test",
            PipelineData::empty(),
            false,
        );
    }

    /// Stored last-result value without scheduling truncation warnings.
    fn last_value(&self) -> Result<Value> {
        // LAST_VARIABLE_ID always resolves (nothing when empty).
        Ok(self.stack.get_var(LAST_VARIABLE_ID, Span::test_data())?)
    }
}

fn seed_pwd(engine_state: &mut EngineState) {
    // Engine rejects trailing separators on PWD.
    let mut cwd = std::env::temp_dir();
    let _ = cwd.pop();
    if cwd.as_os_str().is_empty() {
        cwd = std::path::PathBuf::from("/");
    }
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
}

fn last_var() -> String {
    format!("${LAST_RESULT_VAR_NAME}")
}

#[test]
fn stores_successful_result() -> Result {
    let mut session = Interactive::new();
    session.run("1 + 2");
    assert_eq!(session.last_value()?, Value::test_int(3));
    Ok(())
}

#[test]
fn bare_last_does_not_clobber() -> Result {
    let mut session = Interactive::new();
    session.run("[1 2 3]");
    session.run(&last_var());
    assert_eq!(
        session.last_value()?,
        Value::test_list(vec![
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(3),
        ])
    );

    session.run(&last_var());
    assert_eq!(
        session.last_value()?,
        Value::test_list(vec![
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(3),
        ])
    );
    Ok(())
}

#[test]
fn transform_of_last_updates_store() -> Result {
    let mut session = Interactive::new();
    session.run("[10 20 30]");
    session.run(&format!("{} | first", last_var()));
    assert_eq!(session.last_value()?, Value::test_int(10));
    Ok(())
}

#[test]
fn empty_success_overwrites() -> Result {
    let mut session = Interactive::new();
    session.run("42");
    session.run("null");
    assert_eq!(session.last_value()?, Value::test_nothing());
    Ok(())
}

#[test]
fn error_does_not_overwrite() -> Result {
    let mut session = Interactive::new();
    session.run("99");
    session.run("error make {msg: boom}");
    assert_eq!(session.last_value()?, Value::test_int(99));
    Ok(())
}

#[test]
fn zero_budget_disables_capture() -> Result {
    let mut session = Interactive::new().with_last_result_size(Filesize::ZERO);
    session.run("123");
    assert_eq!(session.last_value()?, Value::test_nothing());
    Ok(())
}

#[test]
fn oversized_result_is_truncated_under_budget() -> Result {
    let budget = std::mem::size_of::<Value>() + 32;
    let mut session = Interactive::new().with_last_result_size(Filesize::new(budget as i64));

    let big = "x".repeat(10_000);
    session.run(&format!("\"{big}\""));

    let stored = session.last_value()?;
    assert!(
        stored.memory_size() <= budget,
        "stored memory_size {} exceeds budget {budget}",
        stored.memory_size()
    );
    assert!(session.stack.last_result_was_truncated());

    match stored {
        Value::String { val, .. } => assert!(val.len() < big.len()),
        Value::Nothing { .. } => {} // acceptable if budget is extremely tight
        other => panic!("unexpected stored type: {other:?}"),
    }
    Ok(())
}

#[test]
fn truncated_list_prefix_respects_byte_budget() -> Result {
    let one = Value::test_int(0).memory_size();
    let budget = Value::test_list(vec![]).memory_size() + one * 5;
    let mut session = Interactive::new().with_last_result_size(Filesize::new(budget as i64));

    session.run("0..100 | each {|i| $i}");

    let stored = session.last_value()?;
    assert!(
        stored.memory_size() <= budget,
        "stored memory_size {} exceeds budget {budget}",
        stored.memory_size()
    );
    assert!(session.stack.last_result_was_truncated());
    Ok(())
}

#[test]
fn non_interactive_does_not_capture() -> Result {
    let mut session = Interactive::non_interactive();
    session.run("7");
    assert_eq!(session.last_value()?, Value::test_nothing());
    Ok(())
}

#[test]
fn last_result_var_name_constant_is_used() -> Result {
    assert_eq!(LAST_RESULT_VAR_NAME, "last");

    let mut session = Interactive::new();
    session.run("5");
    session.run("$last");
    assert_eq!(session.last_value()?, Value::test_int(5));
    Ok(())
}

#[test]
fn preserves_pipeline_metadata_for_ls_path_columns() -> Result {
    let mut session = Interactive::new();

    session.run(r#"[{name: "foo.txt"}] | metadata set --path-columns [name]"#);

    let meta = session
        .stack
        .last_result_metadata()
        .expect("last-result should keep pipeline metadata");
    assert_eq!(meta.path_columns, vec!["name".to_string()]);

    session.run(r#"[{name: "bar.txt"}] | metadata set --path-columns [name]"#);
    let data = session.stack.last_result_pipeline_data(Span::test_data());
    let md = data
        .metadata_ref()
        .expect("pipeline data should carry metadata");
    assert_eq!(md.path_columns, vec!["name".to_string()]);
    Ok(())
}

#[test]
fn truncation_warning_fires_across_repl_parent_child_stacks() -> Result {
    // REPL uses Stack::with_parent each iteration; warn flags must be shared via Arc.
    let budget = std::mem::size_of::<Value>() + 16;
    let parent_arc = Arc::new(Stack::new());

    let mut child = Stack::with_parent(parent_arc.clone());
    child.set_last_result(Value::test_string("y".repeat(10_000)), None, budget);
    assert!(child.last_result_was_truncated());
    assert!(child.last_result_warn_pending());

    let merged = Stack::with_changes_from_child(parent_arc, child);
    let next_iter = Stack::with_parent(Arc::new(merged));

    next_iter.defer_last_result_truncation_warning();
    assert!(
        next_iter.take_last_result_warn_deferred(),
        "warn must defer across parent/child so it can print after $last output"
    );
    assert!(
        !next_iter.take_last_result_warn_deferred(),
        "deferred warn is one-shot after take"
    );
    Ok(())
}

#[test]
fn truncation_warning_is_deferred_until_after_print() -> Result {
    let budget = std::mem::size_of::<Value>() + 16;
    let mut stack = Stack::new();

    stack.set_last_result(Value::test_string("z".repeat(5000)), None, budget);
    assert!(stack.last_result_warn_pending());
    assert!(!stack.take_last_result_warn_deferred());

    stack.defer_last_result_truncation_warning();
    assert!(!stack.last_result_warn_pending());
    assert!(stack.take_last_result_warn_deferred());
    Ok(())
}

#[test]
fn engine_stats_reports_last_result_sizes() -> Result {
    let budget = 4096i64;
    let mut session = Interactive::new().with_last_result_size(Filesize::new(budget));

    session.run("1..20 | each {|i| $i}");

    let scope = nu_engine::scope::ScopeData::new(&session.engine_state, &session.stack);
    let stats = scope.collect_engine_state(Span::test_data());
    let last = stats
        .get_data_by_key("last_result")
        .expect("last_result field on engine-stats");

    assert_eq!(
        last.get_data_by_key("size_limit")
            .expect("size_limit")
            .as_filesize()
            .expect("filesize"),
        Filesize::new(budget)
    );

    let mem = last
        .get_data_by_key("memory_size")
        .expect("memory_size")
        .as_filesize()
        .expect("filesize");
    assert!(mem.get() > 0);
    assert!(mem.get() as usize <= budget as usize || session.stack.last_result_was_truncated());

    assert_eq!(
        last.get_data_by_key("name")
            .expect("name")
            .as_str()
            .expect("string"),
        last_var()
    );
    Ok(())
}

/// Cross-platform external that prints `marker` with no trailing newline.
fn external_print_cmd(marker: &str) -> String {
    // Prefer the workspace `nu --testbin nonu` when the binary is present (CI /
    // full builds). Fall back to a system utility so `cargo test -p nu-cli` alone
    // still exercises the capture path.
    let nu = nu_test_support::fs::executable_path();
    if nu.exists() {
        let nu_path = nu.display().to_string().replace('\\', "/");
        return format!(r#"^"{nu_path}" --testbin nonu {marker}"#);
    }

    #[cfg(windows)]
    {
        // `cmd /c <nul set /p=` prints without a newline.
        format!(r#"^cmd /c "<nul set /p={marker}""#)
    }
    #[cfg(not(windows))]
    {
        format!(r#"^/usr/bin/printf '{marker}'"#)
    }
}

#[test]
fn external_command_stdout_is_stored_as_binary() -> Result {
    // Bare external commands inherit the terminal unless interactive last-result
    // capture forces a pipe. Without that, `$last` was empty binary.
    let marker = "last_external_marker_xyz";
    let mut session = Interactive::new();
    session.run(&external_print_cmd(marker));

    let stored = session.last_value()?;
    match stored {
        Value::Binary { val, .. } => {
            assert_eq!(
                val.as_ref(),
                marker.as_bytes(),
                "external stdout should be stored as binary bytes, got {val:?}"
            );
        }
        other => panic!("expected binary $last from external, got {other:?}"),
    }
    Ok(())
}

#[test]
fn zero_budget_does_not_force_external_capture() -> Result {
    let mut session = Interactive::new().with_last_result_size(Filesize::ZERO);
    session.run(&external_print_cmd("should_not_store"));
    assert_eq!(session.last_value()?, Value::test_nothing());
    Ok(())
}
