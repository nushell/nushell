//! Embeds nushell: builds the EngineState, parses the target script, and
//! runs it on a dedicated thread with the DapDebugger activated.

use crate::dap::protocol::DapWriter;
use crate::dap::types::{Breakpoint, DapEvent, LaunchArgs, Source};
use crate::debugger::DapDebugger;
use crate::state::DebugState;
use nu_protocol::ast::Block;
use nu_protocol::debugger::WithDebug;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{PipelineData, Signals, Span, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// Start one run of the target script. `engine_state` is the host's engine,
/// already cloned by the caller for this run (see [`prepare_engine`]).
pub(crate) fn spawn_eval_thread(
    launch: LaunchArgs,
    state: Arc<DebugState>,
    writer: DapWriter,
    engine_state: EngineState,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("nu-eval".into())
        // nu programs can recurse; give the eval thread a generous stack.
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let state_for_exit = state.clone();

            // A panic in evaluation must not leave the session hung with no
            // terminated event.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(launch, state, &writer, engine_state)
            }));

            let mut exit_code = 0;

            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(msg)) => {
                    exit_code = 1;
                    writer.output("stderr", format!("nu-dap: {msg}\n"));
                }
                Err(panic) => {
                    exit_code = 1;
                    let msg = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".into());
                    writer.output("stderr", format!("nu-dap: internal error (panic): {msg}\n"));
                }
            }

            // A restart replaces this thread within the same DAP session, so
            // announcing termination here would end that session.
            if !state_for_exit.is_restarting() {
                // Late output must reach the client before termination.
                crate::stdio::flush_output(std::time::Duration::from_secs(2));
                writer.event(DapEvent::Terminated);
                writer.event(DapEvent::Exited { exit_code });
            }
        })
        .expect("spawn eval thread")
}

/// One debug session, start to finish. The steps are ordered phases, hence
/// the flat reading order rather than fewer, fatter functions.
fn run(
    launch: LaunchArgs,
    state: Arc<DebugState>,
    writer: &DapWriter,
    engine_state: EngineState,
) -> Result<(), String> {
    let target = Target::resolve(&launch)?;
    target.enter_cwd();

    let mut engine_state = prepare_engine(engine_state, &target, &state, writer)?;
    let block = parse_script(&mut engine_state, &target)?;

    cache_render_facts(&engine_state, &state);
    publish_valid_positions(&engine_state, &block, &state, writer);

    // After the parse, so the script's own `def`s are in scope, and before
    // `activate_debugger`, so the clone starts out undebugged.
    *state.scratch.lock() = Some(crate::eval_scratch::Scratch::from_run_engine(&engine_state));

    // Paired with the `deactivate_debugger` further down.
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
    // forwarders.

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
        let base = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let program = nu_path::canonicalize_with(&launch.program, base)
            .map_err(|e| format!("cannot read {}: {e}", launch.program))?;
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

    /// Move the *process* into the target's directory — kept out of `resolve`
    /// because it mutates global state. The cwd matters beyond `$env.PWD`: the
    /// relative paths nu records for `source`/`use` files are canonicalized
    /// against it.
    fn enter_cwd(&self) {
        let _ = std::env::set_current_dir(&self.cwd);
    }
}

/// Make one run's engine out of the host's fully built one, adjusting only
/// what a debug session needs differently, so the debugged script sees the
/// same nushell as `nu script.nu` would give.
///
/// Cloning matters: parsing the target merges its decls into the engine and a
/// `restart` must not inherit them, so every run starts from the pristine
/// template.
fn prepare_engine(
    mut engine_state: EngineState,
    target: &Target,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) -> Result<EngineState, String> {
    // Several fields survive a clone as shared `Arc`s, the debugger slot most
    // of all: without this a `restart` would activate the new run's debugger
    // in the slot the outgoing run then deactivates, and the fresh run would
    // silently lose its breakpoints.
    engine_state.make_session_state_unique();

    engine_state = register_dap_commands(engine_state, state, writer)?;

    // A per-run interrupt flag, not the host's: a flag left raised by one run
    // would abort the next one instantly on `restart`.
    engine_state.set_signals(Signals::new(Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    ))));

    engine_state.add_env_var(
        String::from("PWD"),
        Value::string(&target.cwd, Span::unknown()),
    );

    Ok(engine_state)
}

/// The host's `print` and `input` family write to a terminal, but our stdout
/// is the DAP wire. Registering last means these shims (print_cmd.rs) shadow
/// them for everything parsed afterwards.
fn register_dap_commands(
    mut engine_state: EngineState,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) -> Result<EngineState, String> {
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

    Ok(engine_state)
}

/// Parse the target script and merge it into the engine. A parse or compile
/// error is fatal: nothing downstream can run.
fn parse_script(engine_state: &mut EngineState, target: &Target) -> Result<Arc<Block>, String> {
    let mut working_set = StateWorkingSet::new(engine_state);

    let block = nu_parser::parse(
        &mut working_set,
        Some(&target.program.to_string_lossy()),
        &target.contents,
        false,
    );

    if let Some(err) = working_set.parse_errors.first() {
        // Display, not Debug, and matching the compile-error arm below: the
        // Debug form dumps spans into the message the user actually reads.
        return Err(format!("parse error: {err}"));
    }

    if let Some(err) = working_set.compile_errors.first() {
        return Err(format!("compile error: {err}"));
    }

    let delta = working_set.render();
    engine_state
        .merge_delta(delta)
        .map_err(|e| format!("merge_delta: {e:?}"))?;

    Ok(block)
}

/// Closure source text and capture names, resolved once now that every block
/// exists — the server thread can't reach `engine_state` to do it later (see
/// the concurrency rule in state.rs).
fn cache_render_facts(engine_state: &EngineState, state: &DebugState) {
    *state.cache.lock() = Arc::new(crate::variables::collect_render_cache(engine_state));
}

/// Run the script: top-level code first, then an entry point if there is one.
/// The entry point's output supersedes the top-level result when it runs.
fn eval_program(
    engine_state: &mut EngineState,
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
/// (for no-`main` libraries), else `main`. The flag distinguishes them because
/// a missing explicit entry is an error, a missing `main` is not.
fn entry_point(launch: &LaunchArgs) -> (String, bool) {
    match &launch.entry_point {
        Some(name) if !name.trim().is_empty() => (name.trim().to_string(), true),
        _ => ("main".to_string(), false),
    }
}

/// Consume the final pipeline and echo any value it produced.
///
/// Must run *while the debugger is still active*: a lazy pipeline
/// (`ls | each { … }`) runs its closures only when consumed here, so
/// draining after deactivate would fire no breakpoints inside them.
fn drain_final_value(
    result: Result<nu_protocol::PipelineExecutionData, nu_protocol::ShellError>,
    engine_state: &EngineState,
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
        // Interrupted == user hit stop. Matched on the variant rather than
        // scraped out of a `Debug` dump: `Debug` is not a stable interface,
        // and matching its text swallowed any error merely mentioning the
        // word, reporting a failed run as a clean exit.
        Err(nu_protocol::ShellError::Interrupted { .. }) => Ok(()),
        // Display, not Debug: users get the message, not the enum.
        Err(e) => Err(format!("{e}")),
    }
}

/// Call an entry-point command with the launch args, the same contract
/// `nu script.nu args...` provides. `Ok(None)` when there is nothing to call:
/// an `explicit` entry point that is missing is an error, a missing `main` is
/// not.
fn call_entry(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    name: &str,
    args: &[String],
    explicit: bool,
) -> Result<Option<nu_protocol::PipelineExecutionData>, nu_protocol::ShellError> {
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

    // Synthesized with nu's own arg escaping, which keeps `3` an int literal
    // and quotes what must be quoted.
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
            // The common failure is launching without required args; make that
            // actionable instead of dumping a parse error.
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

/// Collect the `(line, column)` of every steppable instruction, per file, and
/// reconcile breakpoints set before parsing: snapped onto a real position or
/// left unverified, either way announced as a `breakpoint` changed event.
fn publish_valid_positions(
    engine_state: &EngineState,
    top_block: &nu_protocol::ast::Block,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) {
    // The session's table, so these ids are the ones `setBreakpoints`
    // interned the client's paths under.
    let mut source_map = crate::source_map::SourceMap::new(state.files.clone());
    source_map.refresh(engine_state);

    let mut valid: HashMap<crate::file_table::FileId, BTreeSet<(i64, i64)>> = HashMap::new();
    let mut collect = |ir: &nu_protocol::ir::IrBlock| {
        for span in &ir.spans {
            if let Some(pos) = source_map.resolve_steppable(*span) {
                valid
                    .entry(pos.file)
                    .or_default()
                    .insert((pos.line as i64, pos.column as i64));
            }
        }
    };
    // parse() returns the script's own block without registering it in the
    // engine, so walk it explicitly...
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
        let mut session = state.session_state.lock();
        session.valid_positions = valid;
        session.parse_done = true;

        for (file, bps) in session.breakpoints.clone() {
            let mut changed = std::collections::BTreeMap::new();
            for (requested, mut props) in bps {
                let (snapped, verified) = session.snap(file, requested.line, requested.column);
                props.verified = verified;
                // One breakpoint per position: on collision the first wins and
                // the loser is dropped, announced unverified so the client
                // greys it out rather than showing a marker that can't hit.
                if changed.contains_key(&snapped) {
                    events.push((props.id, false, requested, file, Some(snapped)));
                    continue;
                }
                if snapped != requested || !verified {
                    events.push((props.id, verified, snapped, file, None));
                }
                changed.insert(snapped, props);
            }
            session.breakpoints.insert(file, changed);
        }
    }
    for (id, verified, pos, file, collided_with) in events {
        let breakpoint = Breakpoint {
            id: Some(id),
            verified,
            line: pos.line,
            column: pos.column,
            source: Some(Source {
                name: None,
                // The client needs a path to place the marker; the id is ours.
                path: Some(source_map.path(file)),
            }),
            // Only set for the loser of a collision, as the greying reason.
            message: collided_with.map(|at| format!("another breakpoint already covers {at}")),
        };
        writer.event(DapEvent::Breakpoint {
            reason: "changed",
            breakpoint,
        });
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::engine`].

    use super::{Target, parse_script};
    use nu_protocol::engine::EngineState;

    /// A target that never touches the filesystem: parsing only needs bytes.
    fn target(contents: &str) -> Target {
        Target {
            program: std::path::PathBuf::from("script.nu"),
            contents: contents.as_bytes().to_vec(),
            cwd: ".".into(),
        }
    }

    /// Baseline for the test below. Plain arithmetic, because a bare
    /// `EngineState` carries no declarations — not even `let`.
    #[test]
    fn valid_script_parses() {
        let mut engine_state = EngineState::new();
        parse_script(&mut engine_state, &target("1 + 1\n")).expect("parses");
    }

    /// Parse and compile errors are separate channels, and only the parse side
    /// is obvious. `$env.PWD = ...` parses fine and fails to compile; unreported,
    /// it would reach `eval_block` with no `ir_block` and the user would see
    /// nushell's "block is missing compiled representation" instead.
    #[test]
    fn compile_error_is_reported_as_a_launch_failure() {
        let mut engine_state = EngineState::new();
        let err = parse_script(&mut engine_state, &target("$env.PWD = \"/tmp\"\n"))
            .expect_err("compile error");

        assert!(
            err.starts_with("compile error:"),
            "should be reported as a compile error, not a parse error: {err}"
        );

        assert!(
            err.contains("PWD cannot be set manually"),
            "should carry nushell's own message: {err}"
        );
    }
}
