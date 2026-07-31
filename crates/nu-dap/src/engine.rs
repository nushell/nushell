//! Embeds nushell: builds the EngineState, parses the target script, and
//! runs it on a dedicated thread with the DapDebugger activated.

use crate::dap::protocol::DapWriter;
use crate::dap::types::LaunchArgs;
use crate::debugger::DapDebugger;
use crate::state::DebugState;
use nu_protocol::debugger::WithDebug;
use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Value};
use serde_json::json;
use std::sync::Arc;

pub(crate) fn spawn_eval_thread(
    launch: LaunchArgs,
    state: Arc<DebugState>,
    writer: DapWriter,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("nu-eval".into())
        // nu programs can recurse; give the eval thread a generous stack.
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let state_for_exit = state.clone();
            // A panic anywhere in evaluation must not leave the session hung
            // with no terminated event — catch it and report.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(launch, state, &writer)
            }));
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(msg)) => writer.output("stderr", format!("nu-dap: {msg}\n")),
                Err(panic) => {
                    let msg = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".into());
                    writer.output("stderr", format!("nu-dap: internal error (panic): {msg}\n"));
                }
            }
            // A restart replaces this thread with a fresh run in the same DAP
            // session; announcing termination here would end the session.
            if !state_for_exit.is_restarting() {
                // Late output (an external's last lines, a final drain) must
                // reach the client before we announce termination.
                crate::stdio::flush_output(std::time::Duration::from_secs(2));
                writer.event("terminated", json!({}));
                writer.event("exited", json!({ "exitCode": 0 }));
            }
        })
        .expect("spawn eval thread")
}

fn run(launch: LaunchArgs, state: Arc<DebugState>, writer: &DapWriter) -> Result<(), String> {
    // Canonical but NOT verbatim (\\?\): verbatim paths break nu's path
    // joining when the script `source`s siblings. See paths.rs.
    let program = std::path::PathBuf::from(crate::paths::canonical_str(&launch.program));
    let contents =
        std::fs::read(&program).map_err(|e| format!("cannot read {}: {e}", program.display()))?;

    let cwd = launch
        .cwd
        .clone()
        .or_else(|| program.parent().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".into());

    // The process cwd matters beyond $env.PWD: relative paths that nu records
    // for `source`/`use` files are canonicalized against it (source_map.rs).
    let _ = std::env::set_current_dir(&cwd);

    // --- Engine state: core language + full builtin command set ---
    let mut engine_state = nu_cmd_lang::create_default_context();
    engine_state = nu_command::add_shell_command_context(engine_state);

    // `print` renders records via the `table` command only if this id is set
    // (nu-cli sets it at startup; without it, printing a record fails).
    engine_state.table_decl_id = engine_state.find_decl(b"table", &[]);

    // Populate the `$nu` constant (paths, pid, os-info, …); else it's empty.
    engine_state.generate_nu_constant();

    // `print`/`input` live in nu-cli, which we don't embed, so the parser would
    // treat them as externals. Register our own DAP-aware shims (print_cmd.rs).
    {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(crate::print_cmd::DapPrint {
            writer: writer.clone(),
        }));
        working_set.add_decl(Box::new(crate::print_cmd::DapInput {
            state: state.clone(),
            writer: writer.clone(),
        }));
        working_set.add_decl(Box::new(crate::print_cmd::DapInputList {
            state: state.clone(),
            writer: writer.clone(),
        }));
        working_set.add_decl(Box::new(crate::print_cmd::DapInputUnsupported {
            name: "input listen",
        }));
        let delta = working_set.render();
        engine_state
            .merge_delta(delta)
            .map_err(|e| format!("register print/input: {e:?}"))?;
    }

    // A fresh EngineState carries Signals::empty(), on which trigger() is a
    // NO-OP. Install a real interrupt flag or terminate/stop will not work.
    engine_state.set_signals(nu_protocol::Signals::new(std::sync::Arc::new(
        std::sync::atomic::AtomicBool::new(false),
    )));

    // Minimal environment. A fuller implementation should mirror what
    // nu-cli's gather_parent_env_vars does (inherit the parent process env).
    for (k, v) in std::env::vars() {
        // Never inherit PWD: shells export it in a format nu rejects as
        // non-absolute (Git Bash `/e/...`), breaking source/use. Set ours below.
        if k.eq_ignore_ascii_case("pwd") {
            continue;
        }

        // nu convention: PATH is a list, not a delimited string.
        let value = if k.eq_ignore_ascii_case("path") {
            Value::list(
                std::env::split_paths(&v)
                    .map(|p| Value::string(p.to_string_lossy(), Span::unknown()))
                    .collect(),
                Span::unknown(),
            )
        } else {
            Value::string(v, Span::unknown())
        };
        engine_state.add_env_var(k, value);
    }

    engine_state.add_env_var(
        "PWD".to_string(),
        Value::string(cwd.clone(), Span::unknown()),
    );

    // --- Parse the target script ---
    let block = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        let block = nu_parser::parse(
            &mut working_set,
            Some(&program.to_string_lossy()),
            &contents,
            false,
        );
        if let Some(err) = working_set.parse_errors.first() {
            return Err(format!("parse error: {err:?}"));
        }
        let delta = working_set.render();
        engine_state
            .merge_delta(delta)
            .map_err(|e| format!("merge_delta: {e:?}"))?;
        block
    };

    // --- Breakpoint verification: which lines actually have instructions? ---
    publish_valid_lines(&engine_state, &block, &state, writer);

    // --- Activate our debugger and evaluate under WithDebug ---
    let dap_debugger = DapDebugger::new(state, writer.clone());
    engine_state
        .activate_debugger(Box::new(dap_debugger))
        .map_err(|e| format!("activate_debugger: {e:?}"))?;

    let mut stack = Stack::new();
    stack.add_env_var("PWD".to_string(), Value::string(cwd, Span::unknown()));

    // Process stdout/stderr were swapped for capture pipes at startup
    // (stdio.rs), so the default Inherit destination already reaches the DAP
    // forwarders — no stack redirection needed.

    let result = nu_engine::eval_block::<WithDebug>(
        &engine_state,
        &mut stack,
        &block,
        PipelineData::empty(),
    );

    // After the top-level code, call an entry point with the launch args: an
    // explicit `entry_point` (for no-`main` libraries), else `main` if defined.
    // A missing explicit entry is an error; a missing `main` means top-level only.
    let (entry_name, explicit) = match &launch.entry_point {
        Some(name) if !name.trim().is_empty() => (name.trim().to_string(), true),
        _ => ("main".to_string(), false),
    };
    let result = match result {
        Ok(top) => match call_entry(
            &mut engine_state,
            &mut stack,
            &entry_name,
            &launch.args,
            explicit,
        ) {
            Ok(Some(out)) => Ok(out),
            Ok(None) => Ok(top),
            Err(e) => Err(e),
        },
        err => err,
    };

    // Drain the final pipeline *while the debugger is still active*: a bare
    // lazy pipeline (`ls | each { … }`) only runs its closures when consumed
    // here, so draining after deactivate would fire no breakpoints inside them.
    // `into_value` can itself raise (error in a closure), so fold it into result.
    let outcome = match result {
        Ok(exec_data) => match exec_data.body.into_value(Span::unknown()) {
            Ok(v) => {
                if !matches!(v, Value::Nothing { .. }) {
                    let s = v.to_expanded_string("\n", engine_state.get_config());
                    if !s.is_empty() {
                        writer.output("stdout", format!("{s}\n"));
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    };

    drop(stack);
    let _ = engine_state.deactivate_debugger();

    match outcome {
        Ok(()) => Ok(()),
        Err(e) => {
            // Interrupted == user hit stop; not an error worth shouting about.
            if format!("{e:?}").contains("Interrupted") {
                Ok(())
            } else {
                // Display, not Debug: users get the message, not the enum.
                Err(format!("{e}"))
            }
        }
    }
}

/// Call an entry-point command (`main`, or an explicitly chosen function) with
/// the launch args — the same contract `nu script.nu args...` provides.
/// Returns Ok(None) when there is nothing to call. `explicit` distinguishes a
/// user-chosen entry point (missing → error) from the implicit `main` default
/// (missing → just run top-level).
fn call_entry(
    engine_state: &mut nu_protocol::engine::EngineState,
    stack: &mut Stack,
    name: &str,
    args: &[String],
    explicit: bool,
) -> Result<Option<nu_protocol::PipelineExecutionData>, nu_protocol::ShellError> {
    use nu_protocol::shell_error::generic::GenericError;

    // Only a command the script itself defined (block-backed decl) counts.
    let decl_ok = engine_state
        .find_decl(name.as_bytes(), &[])
        .filter(|id| engine_state.get_decl(*id).block_id().is_some())
        .is_some();
    if !decl_ok {
        if explicit {
            return Err(nu_protocol::ShellError::Generic(
                GenericError::new_internal(
                    format!("entry point `{name}` is not a command defined by this script"),
                    String::new(),
                ),
            ));
        }
        return Ok(None); // no `main`: top-level only
    }

    // Synthesize `<name> <args...>` with nu's own arg escaping
    // (escape_for_script_arg keeps `3` an int literal, quotes what must be).
    let mut source = String::from(name);
    for a in args {
        source.push(' ');
        source.push_str(&nu_parser::escape_for_script_arg(a));
    }
    let block = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let block = nu_parser::parse(
            &mut working_set,
            Some("<entry-call>"),
            source.as_bytes(),
            false,
        );
        if let Some(err) = working_set.parse_errors.first() {
            // The most common failure is launching without required args —
            // make that actionable instead of dumping a parse error.
            let detail = match err {
                nu_protocol::ParseError::MissingPositional(arg, _, usage) => format!(
                    "`{name}` requires the argument `{arg}` (usage: {}). \
                     Set \"args\" in launch.json, e.g. \"args\": [\"value\"]",
                    usage.trim()
                ),
                other => format!("cannot call `{name}` with these args: {other}"),
            };
            return Err(nu_protocol::ShellError::Generic(
                GenericError::new_internal(detail, String::new()),
            ));
        }
        let delta = working_set.render();
        engine_state.merge_delta(delta)?;
        block
    };
    nu_engine::eval_block::<WithDebug>(engine_state, stack, &block, PipelineData::empty()).map(Some)
}

/// Collect every line carrying a steppable instruction (per file) for
/// breakpoint verification, and reconcile breakpoints set before parsing —
/// snapping to the next valid line (`breakpoint` changed events) or unverifying.
fn publish_valid_lines(
    engine_state: &nu_protocol::engine::EngineState,
    top_block: &nu_protocol::ast::Block,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) {
    use std::collections::{BTreeSet, HashMap};

    let mut source_map = crate::source_map::SourceMap::default();
    source_map.refresh(engine_state);

    let mut valid: HashMap<String, BTreeSet<i64>> = HashMap::new();
    let mut collect = |ir: &nu_protocol::ir::IrBlock| {
        for span in &ir.spans {
            if let Some(pos) = source_map.resolve_steppable(*span) {
                valid.entry(pos.path).or_default().insert(pos.line as i64);
            }
        }
    };
    // The parsed script's own block is returned by parse() but never
    // registered in the engine, so walk it explicitly...
    if let Some(ir) = &top_block.ir_block {
        collect(ir);
    }
    // ...then every registered block (custom command bodies, closures,
    // sourced files).
    for i in 0..engine_state.num_blocks() {
        let block = engine_state.get_block(nu_protocol::BlockId::new(i));
        if let Some(ir) = &block.ir_block {
            collect(ir);
        }
    }

    let mut events = Vec::new();
    {
        let mut inner = state.session_state.lock().expect("session state poisoned");
        inner.valid_lines = valid;
        inner.parse_done = true;

        for (path, bps) in inner.breakpoints.clone() {
            let mut changed = std::collections::BTreeMap::new();
            for (line, mut props) in bps {
                let (snapped, verified) = { inner.snap_line(&path, line) };
                props.verified = verified;
                // On collision (two bps snapping to one line) the first wins.
                let final_line = if changed.contains_key(&snapped) {
                    line
                } else {
                    snapped
                };
                if final_line != line || !verified {
                    events.push((props.id, verified, final_line, path.clone()));
                }
                changed.entry(final_line).or_insert(props);
            }
            inner.breakpoints.insert(path, changed);
        }
    }
    for (id, verified, line, path) in events {
        writer.event(
            "breakpoint",
            json!({
                "reason": "changed",
                "breakpoint": {
                    "id": id,
                    "verified": verified,
                    "line": line,
                    "source": { "path": path },
                },
            }),
        );
    }
}
