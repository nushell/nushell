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

/// One debug session, start to finish. Each step is a phase of the session
/// lifecycle; the order matters and is the reason they read as a list here
/// rather than being folded together.
fn run(launch: LaunchArgs, state: Arc<DebugState>, writer: &DapWriter) -> Result<(), String> {
    let target = Target::resolve(&launch)?;
    target.enter_cwd();

    let mut engine_state = build_engine(&target, &state, writer)?;
    let block = parse_script(&mut engine_state, &target)?;

    cache_render_facts(&engine_state, &state);
    publish_valid_lines(&engine_state, &block, &state, writer);

    // Everything below runs with the debugger attached, so it must be paired
    // with the `deactivate_debugger` further down.
    let dap_debugger = DapDebugger::new(state, writer.clone());
    engine_state
        .activate_debugger(Box::new(dap_debugger))
        .map_err(|e| format!("activate_debugger: {e:?}"))?;

    let mut stack = Stack::new();
    stack.add_env_var(
        "PWD".to_string(),
        Value::string(target.cwd.clone(), Span::unknown()),
    );
    // Process stdout/stderr were swapped for capture pipes at startup
    // (stdio.rs), so the default Inherit destination already reaches the DAP
    // forwarders — no stack redirection needed.

    let result = eval_program(&mut engine_state, &mut stack, &block, &launch);
    let outcome = drain_final_value(result, &engine_state, writer);

    drop(stack);
    let _ = engine_state.deactivate_debugger();

    into_exit(outcome)
}

/// The script to debug, resolved from the launch arguments.
struct Target {
    program: std::path::PathBuf,
    contents: Vec<u8>,
    cwd: String,
}

impl Target {
    fn resolve(launch: &LaunchArgs) -> Result<Self, String> {
        // Canonical but NOT verbatim (\\?\): verbatim paths break nu's path
        // joining when the script `source`s siblings. See paths.rs.
        let program = std::path::PathBuf::from(crate::paths::canonical_str(&launch.program));
        let contents = std::fs::read(&program)
            .map_err(|e| format!("cannot read {}: {e}", program.display()))?;

        let cwd = launch
            .cwd
            .clone()
            .or_else(|| program.parent().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|| ".".into());

        Ok(Self {
            program,
            contents,
            cwd,
        })
    }

    /// Move the *process* into the target's directory. Separate from `resolve`
    /// because it mutates global state: the cwd matters beyond `$env.PWD`, as
    /// relative paths nu records for `source`/`use` files are canonicalized
    /// against it (source_map.rs).
    fn enter_cwd(&self) {
        let _ = std::env::set_current_dir(&self.cwd);
    }
}

/// A full nushell engine: core language, the builtin command set, our command
/// shims, an interrupt flag, and the inherited environment.
fn build_engine(
    target: &Target,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) -> Result<nu_protocol::engine::EngineState, String> {
    let mut engine_state = nu_cmd_lang::create_default_context();
    engine_state = nu_command::add_shell_command_context(engine_state);

    // `print` renders records via the `table` command only if this id is set
    // (nu-cli sets it at startup; without it, printing a record fails).
    engine_state.table_decl_id = engine_state.find_decl(b"table", &[]);

    // Populate the `$nu` constant (paths, pid, os-info, …); else it's empty.
    engine_state.generate_nu_constant();

    register_dap_commands(&mut engine_state, state, writer)?;

    // A fresh EngineState carries Signals::empty(), on which trigger() is a
    // NO-OP. Install a real interrupt flag or terminate/stop will not work.
    engine_state.set_signals(nu_protocol::Signals::new(std::sync::Arc::new(
        std::sync::atomic::AtomicBool::new(false),
    )));

    seed_env(&mut engine_state, &target.cwd);

    Ok(engine_state)
}

/// `print`/`input` live in nu-cli, which we don't embed, so the parser would
/// treat them as externals. Register our own DAP-aware shims (print_cmd.rs).
fn register_dap_commands(
    engine_state: &mut nu_protocol::engine::EngineState,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) -> Result<(), String> {
    let mut working_set = StateWorkingSet::new(engine_state);
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
        .map_err(|e| format!("register print/input: {e:?}"))
}

/// Inherit the parent process environment. Minimal: a fuller implementation
/// should mirror what nu-cli's `gather_parent_env_vars` does.
fn seed_env(engine_state: &mut nu_protocol::engine::EngineState, cwd: &str) {
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

    engine_state.add_env_var("PWD".to_string(), Value::string(cwd, Span::unknown()));
}

/// Parse the target script and merge it into the engine. A parse error here is
/// fatal: nothing downstream can run, so the session never starts.
fn parse_script(
    engine_state: &mut nu_protocol::engine::EngineState,
    target: &Target,
) -> Result<Arc<nu_protocol::ast::Block>, String> {
    let mut working_set = StateWorkingSet::new(engine_state);

    let block = nu_parser::parse(
        &mut working_set,
        Some(&target.program.to_string_lossy()),
        &target.contents,
        false,
    );

    if let Some(err) = working_set.parse_errors.first() {
        return Err(format!("parse error: {err:?}"));
    }

    let delta = working_set.render();
    engine_state
        .merge_delta(delta)
        .map_err(|e| format!("merge_delta: {e:?}"))?;

    Ok(block)
}

/// Closure source text and capture names, resolved once now that every block
/// exists. The server thread can't reach `engine_state` to do this later (see
/// the concurrency rule in state.rs), so it has to be cached up front.
fn cache_render_facts(engine_state: &nu_protocol::engine::EngineState, state: &DebugState) {
    *state.cache.lock().expect("render cache poisoned") =
        Arc::new(crate::variables::collect_render_cache(engine_state));
}

/// Run the script: top-level code first, then an entry point if there is one.
/// The entry point's output supersedes the top-level result when it runs.
fn eval_program(
    engine_state: &mut nu_protocol::engine::EngineState,
    stack: &mut Stack,
    block: &nu_protocol::ast::Block,
    launch: &LaunchArgs,
) -> Result<nu_protocol::PipelineExecutionData, nu_protocol::ShellError> {
    let top =
        nu_engine::eval_block::<WithDebug>(engine_state, stack, block, PipelineData::empty())?;

    let (name, explicit) = entry_point(launch);
    match call_entry(engine_state, stack, &name, &launch.args, explicit)? {
        Some(out) => Ok(out),
        None => Ok(top),
    }
}

/// Which command to call after the top-level code: an explicit `entry_point`
/// (for no-`main` libraries), else `main` if defined. The flag distinguishes
/// the two, because a missing explicit entry is an error while a missing
/// `main` just means top-level only.
fn entry_point(launch: &LaunchArgs) -> (String, bool) {
    match &launch.entry_point {
        Some(name) if !name.trim().is_empty() => (name.trim().to_string(), true),
        _ => ("main".to_string(), false),
    }
}

/// Consume the final pipeline and echo any value it produced.
///
/// Must run *while the debugger is still active*: a bare lazy pipeline
/// (`ls | each { … }`) only runs its closures when consumed here, so draining
/// after deactivate would fire no breakpoints inside them. `into_value` can
/// itself raise (error in a closure), so its failure folds into the result.
fn drain_final_value(
    result: Result<nu_protocol::PipelineExecutionData, nu_protocol::ShellError>,
    engine_state: &nu_protocol::engine::EngineState,
    writer: &DapWriter,
) -> Result<(), nu_protocol::ShellError> {
    let value = result?.body.into_value(Span::unknown())?;
    if !matches!(value, Value::Nothing { .. }) {
        let s = value.to_expanded_string("\n", engine_state.get_config());
        if !s.is_empty() {
            writer.output("stdout", format!("{s}\n"));
        }
    }
    Ok(())
}

/// Map the run's outcome onto the thread's exit status.
fn into_exit(outcome: Result<(), nu_protocol::ShellError>) -> Result<(), String> {
    match outcome {
        Ok(()) => Ok(()),
        // Interrupted == user hit stop; not an error worth shouting about.
        Err(e) if format!("{e:?}").contains("Interrupted") => Ok(()),
        // Display, not Debug: users get the message, not the enum.
        Err(e) => Err(format!("{e}")),
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
        let mut session = state.session_state.lock().expect("session poisoned");
        session.valid_lines = valid;
        session.parse_done = true;

        for (path, bps) in session.breakpoints.clone() {
            let mut changed = std::collections::BTreeMap::new();
            for (line, mut props) in bps {
                let (snapped, verified) = { session.snap_line(&path, line) };
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
            session.breakpoints.insert(path, changed);
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
