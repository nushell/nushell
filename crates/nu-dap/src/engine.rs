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

            // A panic anywhere in evaluation must not leave the session hung
            // with no terminated event — catch it and report.
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

            // A restart replaces this thread with a fresh run in the same DAP
            // session; announcing termination here would end the session.
            if !state_for_exit.is_restarting() {
                // Late output (an external's last lines, a final drain) must
                // reach the client before we announce termination.
                crate::stdio::flush_output(std::time::Duration::from_secs(2));
                writer.event(DapEvent::Terminated);
                writer.event(DapEvent::Exited { exit_code });
            }
        })
        .expect("spawn eval thread")
}

/// One debug session, start to finish. Each step is a phase of the session
/// lifecycle; the order matters and is the reason they read as a list here
/// rather than being folded together.
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
    publish_valid_lines(&engine_state, &block, &state, writer);

    // Must happen here: after the parse, so the script's own `def`s are in
    // scope, and before `activate_debugger`, so the clone starts out
    // undebugged. A `restart` replaces the previous run's scratch.
    *state.scratch.lock().expect("scratch poisoned") =
        Some(crate::eval_scratch::Scratch::from_run_engine(&engine_state));

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
        let program = std::path::PathBuf::from(crate::paths::canonical(&launch.program));
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

/// Make one run's engine out of the host's.
///
/// The host (`nu --dap`) hands us its fully built `EngineState` — core
/// language, builtin commands, plugins, the gathered parent environment, the
/// `$nu` constant. We take a clone of it per run and adjust only what a debug
/// session needs differently, so the debugged script sees the same nushell the
/// user would get from `nu script.nu`.
///
/// Cloning matters: parsing the target merges its decls into the engine, and a
/// `restart` must not inherit them, so every run starts from the pristine
/// template.
fn prepare_engine(
    mut engine_state: EngineState,
    target: &Target,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) -> Result<EngineState, String> {
    // Several fields survive a clone as shared `Arc`s — the debugger slot most
    // of all. Without this, a `restart` would activate the new run's debugger
    // in the same slot the outgoing run then deactivates (or reports its
    // interrupt through), so the fresh run silently loses its breakpoints.
    engine_state.make_session_state_unique();

    engine_state = register_dap_commands(engine_state, state, writer)?;

    // A per-run interrupt flag, not the host's: `terminate`/`stop` trigger it
    // (debugger/mod.rs), and a flag left raised by one run would abort the next
    // one instantly on `restart`.
    engine_state.set_signals(Signals::new(Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    ))));

    engine_state.add_env_var(
        String::from("PWD"),
        Value::string(&target.cwd, Span::unknown()),
    );

    Ok(engine_state)
}

/// `print` lives in nu-cli and `input`/`input list`/`input listen` in
/// nu-command, but neither flavour works here: their output and prompts belong
/// to a terminal, and our stdout is the DAP wire. Registering last means these
/// shims (print_cmd.rs) shadow the host's for everything parsed afterwards.
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
/// error here is fatal: nothing downstream can run, so the session never
/// starts.
fn parse_script(engine_state: &mut EngineState, target: &Target) -> Result<Arc<Block>, String> {
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
/// exists. The server thread can't reach `engine_state` to do this later (see
/// the concurrency rule in state.rs), so it has to be cached up front.
fn cache_render_facts(engine_state: &EngineState, state: &DebugState) {
    *state.cache.lock().expect("render cache poisoned") =
        Arc::new(crate::variables::collect_render_cache(engine_state));
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
    engine_state: &EngineState,
    top_block: &nu_protocol::ast::Block,
    state: &Arc<DebugState>,
    writer: &DapWriter,
) {
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
                // At most one breakpoint per line: on collision (two bps
                // snapping onto one line) the first wins and the loser is
                // dropped — announced as unverified so the client greys it
                // out instead of showing a marker that can never hit.
                if changed.contains_key(&snapped) {
                    events.push((props.id, false, line, path.clone(), Some(snapped)));
                    continue;
                }
                if snapped != line || !verified {
                    events.push((props.id, verified, snapped, path.clone(), None));
                }
                changed.insert(snapped, props);
            }
            session.breakpoints.insert(path, changed);
        }
    }
    for (id, verified, line, path, collided_with) in events {
        let breakpoint = Breakpoint {
            id: Some(id),
            verified,
            line,
            source: Some(Source {
                name: None,
                path: Some(path),
            }),
            // Only set for the loser of a collision: the client greys the
            // marker out and shows this as the reason.
            message: collided_with.map(|at| format!("another breakpoint already covers line {at}")),
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

    /// A target that never touches the filesystem: `Target::resolve` reads the
    /// script from disk, but parsing only needs its bytes.
    fn target(contents: &str) -> Target {
        Target {
            program: std::path::PathBuf::from("script.nu"),
            contents: contents.as_bytes().to_vec(),
            cwd: ".".into(),
        }
    }

    /// The baseline for the test below: a script with neither kind of error.
    /// Plain arithmetic, because a bare `EngineState` carries no declarations —
    /// not even `let`, which comes from nu-cmd-lang.
    #[test]
    fn valid_script_parses() {
        let mut engine_state = EngineState::new();
        parse_script(&mut engine_state, &target("1 + 1\n")).expect("parses");
    }

    /// Parse errors and compile errors are separate channels, and only the parse
    /// side is obvious. `$env.PWD = ...` parses fine and fails to compile; if the
    /// compile error were not reported here the block would reach `eval_block`
    /// with no `ir_block`, and the user would see nushell's internal "block is
    /// missing compiled representation" instead of the real problem.
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
