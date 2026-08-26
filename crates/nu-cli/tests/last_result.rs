//! Interactive last-result (`$ans`) capture tests.
//!
//! Last-result is only stored when `engine_state.is_interactive` is true and evaluation
//! goes through `eval_source` (the REPL path). `NuTester` uses `eval_block` without that
//! hook, so these tests drive `eval_source` directly while following harness conventions:
//! `Result` returns, structured value asserts, and no `nu!`.
//!
//! When payload capture is enabled (`max_last_result_size > 0`), `$ans` is a record
//! `{ last, exit_code, duration, command }`. With size `0`, `$ans` is
//! `{ exit_code, duration, command }` (no `last` field). `command` is the exact last REPL
//! source (same text reedline stores in history). Payload checks use the `last`
//! field; bare `$ans` / `$ans.*` cell-paths must not clobber `.last`.

use nu_cli::eval_source;
use nu_protocol::{
    Filesize, FilesizeUnit, LAST_RESULT_VAR_NAME, LAST_VARIABLE_ID, PipelineData, Span, Value,
    ast::Expr,
    engine::{EngineState, Stack},
    record,
};
use nu_test_support::prelude::*;
use nu_test_support::tester::PATH_ENV_AUTO_LOAD;
use std::sync::Arc;
use std::time::Duration;

/// Interactive engine + stack for last-result capture (REPL-equivalent path).
struct Interactive {
    engine_state: EngineState,
    stack: Stack,
    /// Last `run()` source, used by [`Self::snapshot_metadata`] as `$ans.command`.
    last_source: String,
}

impl Interactive {
    fn new() -> Self {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        engine_state.is_interactive = true;
        seed_env(&mut engine_state);
        // Capture is opt-in (default 0b); enable a budget so these tests exercise storage.
        Self {
            engine_state,
            stack: Stack::new(),
            last_source: String::new(),
        }
        .with_max_last_result_size(
            Filesize::from_unit(1, FilesizeUnit::MiB).expect("1 MiB fits in Filesize"),
        )
    }

    fn non_interactive() -> Self {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        engine_state.is_interactive = false;
        seed_env(&mut engine_state);
        Self {
            engine_state,
            stack: Stack::new(),
            last_source: String::new(),
        }
    }

    fn with_max_last_result_size(mut self, size: Filesize) -> Self {
        let mut cfg = self.engine_state.get_config().as_ref().clone();
        cfg.max_last_result_size = size;
        self.engine_state.config = Arc::new(cfg);
        self
    }

    /// One interactive source unit (like one REPL entry). Returns the eval exit code.
    fn run(&mut self, code: &str) -> i32 {
        // Match REPL `do_run_cmd`: enable capture only for this user line.
        self.last_source = code.to_string();
        self.engine_state.capture_repl_last_result = true;
        let code = eval_source(
            &mut self.engine_state,
            &mut self.stack,
            code.as_bytes(),
            "test",
            PipelineData::empty(),
            false,
        );
        self.engine_state.capture_repl_last_result = false;
        code
    }

    /// Full `$ans` value (record when present, `nothing` when never snapshotted/stored).
    fn ans_value(&self) -> Result<Value> {
        Ok(self.stack.get_var(LAST_VARIABLE_ID, Span::test_data())?)
    }

    /// `$ans.last` payload (or `nothing` when `$ans` is unset or has no `last` field).
    fn last_payload(&self) -> Result<Value> {
        let ans = self.ans_value()?;
        match ans {
            Value::Nothing { .. } => Ok(ans),
            Value::Record { .. } => Ok(ans
                .get_data_by_key("last")
                .unwrap_or_else(Value::test_nothing)),
            other => panic!("expected $ans record or nothing, got {other:?}"),
        }
    }

    /// Match REPL end-of-line snapshot of exit_code / duration / command.
    fn snapshot_metadata(&mut self, duration: Duration) {
        self.stack.snapshot_ans_repl_metadata(
            &self.engine_state,
            duration,
            self.last_source.clone(),
        );
    }

    /// `$ans.command` (or empty string when `$ans` is unset or has no `command` field).
    fn last_command(&self) -> Result<String> {
        let ans = self.ans_value()?;
        match ans {
            Value::Nothing { .. } => Ok(String::new()),
            Value::Record { .. } => Ok(ans
                .get_data_by_key("command")
                .and_then(|v| v.as_str().ok().map(str::to_string))
                .unwrap_or_default()),
            other => panic!("expected $ans record or nothing, got {other:?}"),
        }
    }
}

fn seed_env(engine_state: &mut EngineState) {
    // Engine rejects trailing separators on PWD.
    let mut cwd = std::env::temp_dir();
    let _ = cwd.pop();
    if cwd.as_os_str().is_empty() {
        cwd = std::path::PathBuf::from("/");
    }
    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

    // `#[deps(...)]` registers bin dirs in PATH_ENV_AUTO_LOAD; put them on PATH so
    // external tests can call testbins by name without absolute-path quoting.
    let path: Vec<Value> = PATH_ENV_AUTO_LOAD
        .read()
        .iter()
        .map(|p| Value::test_string(p.to_string_lossy()))
        .collect();
    if !path.is_empty() {
        engine_state.add_env_var("PATH".into(), Value::test_list(path));
    }
}

fn last_var() -> String {
    format!("${LAST_RESULT_VAR_NAME}")
}

#[test]
fn stores_successful_result() -> Result {
    let mut session = Interactive::new();
    session.run("1 + 2");
    assert_eq!(session.last_payload()?, Value::test_int(3));
    let ans = session.ans_value()?;
    assert!(matches!(ans, Value::Record { .. }));
    assert_eq!(
        ans.get_data_by_key("exit_code").expect("exit_code"),
        Value::test_int(0)
    );
    assert_eq!(
        ans.get_data_by_key("duration").expect("duration"),
        Value::test_duration(0)
    );
    Ok(())
}

#[test]
fn bare_ans_does_not_clobber() -> Result {
    let mut session = Interactive::new();
    session.run("[1 2 3]");
    session.run(&last_var());
    assert_eq!(
        session.last_payload()?,
        Value::test_list(vec![
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(3),
        ])
    );

    session.run(&format!("{}.last", last_var()));
    assert_eq!(
        session.last_payload()?,
        Value::test_list(vec![
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(3),
        ])
    );

    session.run(&format!("{}.exit_code", last_var()));
    assert_eq!(
        session.last_payload()?,
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
    session.run(&format!("{}.last | first", last_var()));
    assert_eq!(session.last_payload()?, Value::test_int(10));
    Ok(())
}

#[test]
fn empty_success_overwrites() -> Result {
    let mut session = Interactive::new();
    session.run("42");
    session.run("null");
    assert_eq!(session.last_payload()?, Value::test_nothing());
    Ok(())
}

#[test]
fn error_does_not_overwrite() -> Result {
    let mut session = Interactive::new();
    session.run("99");
    session.run("error make {msg: boom}");
    assert_eq!(session.last_payload()?, Value::test_int(99));
    Ok(())
}

#[test]
fn zero_budget_disables_last_payload_only() -> Result {
    let mut session = Interactive::new().with_max_last_result_size(Filesize::ZERO);
    session.run("123");
    // Without a REPL metadata snapshot, the slot was never marked present.
    assert_eq!(session.ans_value()?, Value::test_nothing());

    session.stack.set_last_exit_code(0, Span::test_data());
    session.snapshot_metadata(Duration::from_millis(10));

    let ans = session.ans_value()?;
    assert!(
        matches!(ans, Value::Record { .. }),
        "expected $ans record with exit_code/duration when budget is 0, got {ans:?}"
    );
    assert!(
        ans.get_data_by_key("last").is_none(),
        "expected no `last` field when budget is 0, got {ans:?}"
    );
    assert_eq!(
        ans.get_data_by_key("exit_code").expect("exit_code"),
        Value::test_int(0)
    );
    assert_eq!(
        ans.get_data_by_key("duration").expect("duration"),
        Value::test_duration(10_000_000)
    );
    assert_eq!(
        ans.get_data_by_key("command").expect("command"),
        Value::test_string("123")
    );
    assert_eq!(session.last_payload()?, Value::test_nothing());
    Ok(())
}

#[test]
fn oversized_result_is_truncated_under_budget() -> Result {
    let budget = std::mem::size_of::<Value>() + 32;
    let mut session = Interactive::new().with_max_last_result_size(Filesize::new(budget as i64));

    let big = "x".repeat(10_000);
    session.run(&format!("\"{big}\""));

    let stored = session.last_payload()?;
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
    let mut session = Interactive::new().with_max_last_result_size(Filesize::new(budget as i64));

    session.run("0..100 | each {|i| $i}");

    let stored = session.last_payload()?;
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
    assert_eq!(session.ans_value()?, Value::test_nothing());
    Ok(())
}

#[test]
fn last_result_var_name_constant_is_used() -> Result {
    assert_eq!(LAST_RESULT_VAR_NAME, "ans");

    let mut session = Interactive::new();
    session.run("5");
    session.run("$ans");
    assert_eq!(session.last_payload()?, Value::test_int(5));
    Ok(())
}

#[test]
fn ans_name_is_reserved() -> Result {
    // `ans` is reserved for interactive last-result; user rebinding is a parse error.
    let engine_state = nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());

    for source in [b"let ans = 1".as_slice(), b"let $ans = 1".as_slice()] {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        let _block = nu_parser::parse(&mut working_set, None, source, false);
        assert!(
            working_set.parse_errors.iter().any(|e| {
                matches!(e, nu_protocol::ParseError::NameIsBuiltinVar(name, _) if name == "ans")
            }),
            "expected NameIsBuiltinVar for `{}`, got {:?}",
            String::from_utf8_lossy(source),
            working_set.parse_errors
        );
    }
    Ok(())
}

#[test]
fn ans_is_not_listed_in_closure_captures() -> Result {
    // `$ans` resolves from the shared last-result slot; capturing it would clone a large
    // `.last` into every closure / `each` body that mentions it.
    let engine_state = nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, b"{ $ans; $ans.last }", false);
    assert!(
        working_set.parse_errors.is_empty(),
        "unexpected parse errors: {:?}",
        working_set.parse_errors
    );

    let mut found_closure = false;
    for pipeline in &block.pipelines {
        for element in &pipeline.elements {
            if let Expr::Closure(block_id) = &element.expr.expr {
                found_closure = true;
                let inner = working_set.get_block(*block_id);
                assert!(
                    !inner.captures.iter().any(|(id, _)| *id == LAST_VARIABLE_ID),
                    "expected `$ans` not in closure captures, got {:?}",
                    inner.captures
                );
            }
        }
    }
    assert!(found_closure, "expected a top-level closure expression");
    Ok(())
}

#[test]
fn snapshot_always_sets_metadata_without_prior_store() -> Result {
    // Every user REPL line should get exit_code/duration even if `.last` was never stored
    // (e.g. first-line error, or budget off until snapshot).
    let mut session = Interactive::new();
    assert_eq!(session.ans_value()?, Value::test_nothing());

    session.stack.set_last_exit_code(9, Span::test_data());
    session.snapshot_metadata(Duration::from_millis(3));

    let ans = session.ans_value()?;
    assert!(matches!(ans, Value::Record { .. }), "got {ans:?}");
    assert!(
        ans.get_data_by_key("last").is_none(),
        "no payload store yet, so last must be absent, got {ans:?}"
    );
    assert_eq!(
        ans.get_data_by_key("exit_code").expect("exit_code"),
        Value::test_int(9)
    );
    assert_eq!(
        ans.get_data_by_key("duration").expect("duration"),
        Value::test_duration(3_000_000)
    );
    Ok(())
}

#[test]
fn underscore_is_not_reserved() -> Result {
    // `_` was previously reserved for last-result; it must be bindable again.
    let engine_state = nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());

    for source in [b"let _ = 1".as_slice(), b"let $_ = 1".as_slice()] {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        let _block = nu_parser::parse(&mut working_set, None, source, false);
        assert!(
            working_set.parse_errors.is_empty(),
            "expected `_` to be bindable, got {:?}",
            working_set.parse_errors
        );
    }
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
        "warn must defer across parent/child so it can print after $ans output"
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
    let mut session = Interactive::new().with_max_last_result_size(Filesize::new(budget));

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

#[test]
#[deps(TESTBIN_NONU)]
fn bare_external_with_inherited_stdout_does_not_capture() -> Result {
    // Bare externals keep stdout on the TTY (Print/Inherit). Forcing a pipe for
    // `$ans` would hang interactive tools (nvim, btm). Prior `.last` is left alone.
    let mut session = Interactive::new();
    session.run("42");
    assert_eq!(session.last_payload()?, Value::test_int(42));

    session.run("^nonu should_not_overwrite_last");
    assert_eq!(
        session.last_payload()?,
        Value::test_int(42),
        "bare external must not steal TTY stdout or clobber $ans.last"
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_NONU)]
fn external_command_stdout_is_stored_when_piped() -> Result {
    // External UTF-8 bytes enter `$ans.last` only when already in the pipeline.
    // Structured internal results still take the Value/ListStream paths unchanged.
    // Testbins live in `crates/testbins` and are built via `#[deps(TESTBIN_*)]`.
    let marker = "last_external_marker_xyz";
    let mut session = Interactive::new();
    // `nonu` prints args with no trailing newline; `| collect` pipes stdout.
    session.run(&format!("^nonu {marker} | collect"));

    assert_eq!(session.last_payload()?, Value::test_string(marker));
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn external_command_trailing_newline_is_trimmed_in_last() -> Result {
    // Match ByteStream::into_value / complete: one trailing newline is stripped.
    // `cococo` uses println! (trailing `\n`). Pipe so bytes are capturable.
    let mut session = Interactive::new();
    session.run("^cococo trim_me | collect");

    let stored = session.last_payload()?;
    match stored {
        Value::String { val, .. } => {
            assert_eq!(&*val, "trim_me");
            assert!(!val.ends_with('\n'));
        }
        other => panic!("expected string $ans.last from external, got {other:?}"),
    }
    Ok(())
}

#[test]
fn non_utf8_byte_stream_stays_binary_in_last() -> Result {
    // Explicit binary values must not be force-decoded to string.
    let mut session = Interactive::new();
    session.run("0x[deadbeef]");
    assert_eq!(
        session.last_payload()?,
        Value::test_binary(vec![0xde, 0xad, 0xbe, 0xef])
    );
    Ok(())
}

#[test]
fn internal_structured_last_is_not_stringified() -> Result {
    // Internal table/list values stay structured — auto-decode is byte-stream only.
    let mut session = Interactive::new();
    session.run("[{a: 1} {a: 2}]");
    assert_eq!(
        session.last_payload()?,
        Value::test_list(vec![
            Value::test_record(record! { "a" => Value::test_int(1) }),
            Value::test_record(record! { "a" => Value::test_int(2) }),
        ])
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_NONU)]
fn zero_budget_does_not_force_external_capture() -> Result {
    let mut session = Interactive::new().with_max_last_result_size(Filesize::ZERO);
    session.run("^nonu should_not_store");
    // External capture is skipped when budget is 0; still no `.last` after snapshot.
    session.snapshot_metadata(Duration::from_millis(1));
    let ans = session.ans_value()?;
    assert!(matches!(ans, Value::Record { .. }), "got {ans:?}");
    assert!(
        ans.get_data_by_key("last").is_none(),
        "budget 0 must not store external output in last, got {ans:?}"
    );
    Ok(())
}

#[test]
fn snapshot_ans_repl_metadata_sets_exit_code_and_duration() -> Result {
    let mut session = Interactive::new();
    session.run("42");
    session.stack.set_last_exit_code(7, Span::test_data());
    session.snapshot_metadata(Duration::from_millis(25));

    let ans = session.ans_value()?;
    assert_eq!(session.last_payload()?, Value::test_int(42));
    assert_eq!(
        ans.get_data_by_key("exit_code").expect("exit_code"),
        Value::test_int(7)
    );
    assert_eq!(
        ans.get_data_by_key("duration").expect("duration"),
        Value::test_duration(25_000_000) // 25ms in nanoseconds
    );
    assert_eq!(
        ans.get_data_by_key("command").expect("command"),
        Value::test_string("42")
    );
    Ok(())
}

#[test]
fn snapshot_with_zero_budget_drops_last_keeps_metadata() -> Result {
    let mut session = Interactive::new();
    session.run("1");
    assert!(matches!(session.ans_value()?, Value::Record { .. }));
    assert_eq!(session.last_payload()?, Value::test_int(1));

    let mut cfg = session.engine_state.get_config().as_ref().clone();
    cfg.max_last_result_size = Filesize::ZERO;
    session.engine_state.config = Arc::new(cfg);

    session.stack.set_last_exit_code(3, Span::test_data());
    session.snapshot_metadata(Duration::from_millis(5));

    let ans = session.ans_value()?;
    assert!(
        matches!(ans, Value::Record { .. }),
        "expected $ans to remain a record, got {ans:?}"
    );
    assert!(
        ans.get_data_by_key("last").is_none(),
        "expected `last` removed when budget becomes 0, got {ans:?}"
    );
    assert_eq!(
        ans.get_data_by_key("exit_code").expect("exit_code"),
        Value::test_int(3)
    );
    assert_eq!(
        ans.get_data_by_key("duration").expect("duration"),
        Value::test_duration(5_000_000)
    );
    assert_eq!(
        ans.get_data_by_key("command").expect("command"),
        Value::test_string("1")
    );
    // Payload memory must be gone.
    assert_eq!(session.stack.last_result_memory_size(), 0);
    Ok(())
}

#[test]
fn ls_then_ans_with_config_env() -> Result {
    // Reproduce REPL: enable budget via $env.config, run ls, read $ans.
    let mut session = Interactive::new().with_max_last_result_size(Filesize::ZERO);
    // Budget off first
    assert_eq!(session.ans_value()?, Value::test_nothing());

    // Enable like a user would (also exercises update_config path)
    session.run("$env.config.max_last_result_size = 1mb");
    // Config assignment may store empty/null last; should be present once budget > 0 after store
    session.run("1 + 1");
    let after_math = session.ans_value()?;
    assert!(
        matches!(after_math, Value::Record { .. }),
        "expected record after enabling budget and running math, got {after_math:?}"
    );
    assert_eq!(session.last_payload()?, Value::test_int(2));

    session.run("ls");
    let ans = session.ans_value()?;
    assert!(
        matches!(ans, Value::Record { .. }),
        "expected $ans record after ls, got {ans:?}"
    );
    let last = session.last_payload()?;
    assert!(
        !matches!(last, Value::Nothing { .. }),
        "expected $ans.last after ls, got {last:?}"
    );
    Ok(())
}

#[test]
fn default_zero_budget_means_no_last_field() -> Result {
    let mut session = Interactive::new().with_max_last_result_size(Filesize::ZERO);
    session.run("ls");
    // Store path with budget 0 does not invent a record by itself.
    assert_eq!(session.ans_value()?, Value::test_nothing());

    session.snapshot_metadata(Duration::from_millis(2));
    let ans = session.ans_value()?;
    assert!(matches!(ans, Value::Record { .. }), "got {ans:?}");
    assert!(
        ans.get_data_by_key("last").is_none(),
        "ls must not populate last when budget is 0, got {ans:?}"
    );
    assert_eq!(
        ans.get_data_by_key("command").expect("command"),
        Value::test_string("ls")
    );
    assert_eq!(session.stack.last_result_memory_size(), 0);
    Ok(())
}

#[test]
fn ans_via_source_after_ls() -> Result {
    let mut session = Interactive::new();
    session.run("ls");
    // Evaluate $ans the same way the REPL does, then inspect stored slot still intact
    session.run("$ans");
    let ans = session.ans_value()?;
    assert!(matches!(ans, Value::Record { .. }), "got {ans:?}");
    // $ans.last must still be the ls result (bare $ans must not clobber)
    let last = session.last_payload()?;
    assert!(
        matches!(last, Value::List { .. }),
        "expected list in last after bare $ans, got {last:?}"
    );
    session.run("$ans.last");
    let last2 = session.last_payload()?;
    assert!(
        matches!(last2, Value::List { .. }),
        "expected list still after $ans.last, got {last2:?}"
    );
    Ok(())
}

#[test]
fn command_records_single_command_source() -> Result {
    let mut session = Interactive::new();
    session.run("1 + 2");
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_payload()?, Value::test_int(3));
    assert_eq!(session.last_command()?, "1 + 2");
    Ok(())
}

#[test]
fn command_preserves_multiline_pipeline() -> Result {
    let mut session = Interactive::new();
    let code = "ls\n    | where type == file";
    session.run(code);
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_command()?, code);
    Ok(())
}

#[test]
fn command_updates_on_runtime_error_without_clobbering_last() -> Result {
    let mut session = Interactive::new();
    session.run("99");
    session.snapshot_metadata(Duration::from_millis(1));
    session.run("error make {msg: boom}");
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_payload()?, Value::test_int(99));
    assert_eq!(session.last_command()?, "error make {msg: boom}");
    Ok(())
}

#[test]
fn command_updates_on_parse_error() -> Result {
    let mut session = Interactive::new();
    session.run("1 + 2");
    session.snapshot_metadata(Duration::from_millis(1));
    session.run("let");
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_payload()?, Value::test_int(3));
    assert_eq!(session.last_command()?, "let");
    Ok(())
}

#[test]
fn command_updates_on_bare_ans_without_clobbering_last() -> Result {
    let mut session = Interactive::new();
    session.run("[1 2 3]");
    session.snapshot_metadata(Duration::from_millis(1));
    session.run(&last_var());
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(
        session.last_payload()?,
        Value::test_list(vec![
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(3),
        ])
    );
    assert_eq!(session.last_command()?, last_var());
    Ok(())
}

#[test]
fn error_values_from_str_length_do_not_clobber_last() -> Result {
    // https://github.com/nushell/nushell/issues/18861
    // `str length` embeds type errors as values; `$ans.last` must stay the prior table.
    let mut session = Interactive::new();
    session.run("[{name: a}]");
    let original = session.last_payload()?;
    assert!(
        matches!(original, Value::List { .. }),
        "expected table payload, got {original:?}"
    );

    let failed = format!("{}.last | str length", last_var());
    session.run(&failed);
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_payload()?, original);
    assert_eq!(session.last_command()?, failed);

    let ans_code = session.run(&last_var());
    assert_eq!(
        ans_code, 0,
        "bare $ans must stay printable after error values"
    );
    assert_eq!(session.last_payload()?, original);
    Ok(())
}

#[test]
fn error_values_do_not_clobber_last_with_table_expand_hook() -> Result {
    // Default display_output uses `table -e` on wide terminals; force it so CI reproduces.
    let mut session = Interactive::new();
    session.run("$env.config.hooks.display_output = 'table --expand'");
    session.run("[{name: a}]");
    let original = session.last_payload()?;

    session.run(&format!("{}.last | str length", last_var()));
    assert_eq!(session.last_payload()?, original);

    let ans_code = session.run(&last_var());
    assert_eq!(
        ans_code, 0,
        "bare $ans with table --expand must not rethrow stored type errors"
    );
    assert_eq!(session.last_payload()?, original);
    Ok(())
}

#[test]
fn error_only_list_stream_does_not_clobber_last() -> Result {
    let mut session = Interactive::new();
    session.run("[{name: a}]");
    let original = session.last_payload()?;

    // Range + each yields a list stream of records; str length maps it to error values.
    session.run("1..2 | each {|i| {name: $i}} | str length");
    assert_eq!(session.last_payload()?, original);
    Ok(())
}

#[test]
fn mixed_str_length_result_does_replace_last() -> Result {
    // Cell-path `str length` keeps the table shape: one row becomes an int, one an error value.
    // That mixed list must still replace `$ans.last` (not treated as error-only).
    let mut session = Interactive::new();
    session.run("[{name: 'ab'} {name: 1}]");
    session.run(&format!("{}.last | str length name", last_var()));
    let last = session.last_payload()?;
    let Value::List { vals, .. } = last else {
        panic!("expected list last payload, got {last:?}");
    };
    assert_eq!(vals.len(), 2);
    let name0 = vals[0]
        .as_record()
        .expect("first row")
        .get("name")
        .expect("name");
    assert_eq!(name0, &Value::test_int(2));
    assert!(
        vals[1].is_error(),
        "row with a non-string cell should be an error value, got {:?}",
        vals[1]
    );
    Ok(())
}

#[test]
fn command_updates_when_transforming_last() -> Result {
    let mut session = Interactive::new();
    session.run("[10 20 30]");
    session.snapshot_metadata(Duration::from_millis(1));
    let transform = format!("{}.last | first", last_var());
    session.run(&transform);
    session.snapshot_metadata(Duration::from_millis(1));
    assert_eq!(session.last_payload()?, Value::test_int(10));
    assert_eq!(session.last_command()?, transform);
    Ok(())
}
