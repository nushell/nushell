use crate::{
    completions::NuCompleter,
    nu_highlight::NoOpHighlighter,
    prompt_update,
    reedline_config::{add_menus, create_keybindings, KeybindingsMode},
    util::eval_source,
    NuHighlighter, NuValidator, NushellPrompt,
};
use crossterm::cursor::SetCursorStyle;
use log::{error, trace, warn};
use miette::{ErrReport, IntoDiagnostic, Result};
use nu_cmd_base::{
    hook::eval_hook,
    util::{get_editor, get_guaranteed_cwd},
};
use nu_color_config::StyleComputer;
use nu_engine::{convert_env_values, env_to_strings};
use nu_parser::{lex, parse, trim_quotes_str};
use nu_protocol::{
    config::NuCursorShape,
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::create_nu_constant,
    report_error_new, HistoryConfig, HistoryFileFormat, PipelineData, ShellError, Span, Spanned,
    Value, NU_VARIABLE_ID,
};
use nu_utils::{
    filesystem::{have_permission, PermissionResult},
    utils::perf,
};
use reedline::{
    CursorConfig, CwdAwareHinter, DefaultCompleter, EditCommand, Emacs, FileBackedHistory,
    HistorySessionId, Reedline, SqliteBackedHistory, Vi,
};
use std::{
    collections::HashMap,
    env::temp_dir,
    io::{self, IsTerminal, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use sysinfo::System;

// According to Daniel Imms @Tyriar, we need to do these this way:
// <133 A><prompt><133 B><command><133 C><command output>
// These first two have been moved to prompt_update to get as close as possible to the prompt.
// const PRE_PROMPT_MARKER: &str = "\x1b]133;A\x1b\\";
// const POST_PROMPT_MARKER: &str = "\x1b]133;B\x1b\\";
const PRE_EXECUTE_MARKER: &str = "\x1b]133;C\x1b\\";
// This one is in get_command_finished_marker() now so we can capture the exit codes properly.
// const CMD_FINISHED_MARKER: &str = "\x1b]133;D;{}\x1b\\";
const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

/// The main REPL loop, including spinning up the prompt itself.
pub fn evaluate_repl(
    engine_state: &mut EngineState,
    stack: Stack,
    nushell_path: &str,
    prerun_command: Option<Spanned<String>>,
    load_std_lib: Option<Spanned<String>>,
    entire_start_time: Instant,
) -> Result<()> {
    // throughout this code, we hold this stack uniquely.
    // During the main REPL loop, we hand ownership of this value to an Arc,
    // so that it may be read by various reedline plugins. During this, we
    // can't modify the stack, but at the end of the loop we take back ownership
    // from the Arc. This lets us avoid copying stack variables needlessly
    let mut unique_stack = stack;
    let config = engine_state.get_config();
    let use_color = config.use_ansi_coloring;

    confirm_stdin_is_terminal()?;

    let mut entry_num = 0;

    let shell_integration = config.shell_integration;
    let nu_prompt = NushellPrompt::new(shell_integration);

    let start_time = std::time::Instant::now();
    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, &unique_stack) {
        report_error_new(engine_state, &e);
    }
    perf(
        "translate env vars",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    // seed env vars
    unique_stack.add_env_var(
        "CMD_DURATION_MS".into(),
        Value::string("0823", Span::unknown()),
    );

    unique_stack.add_env_var("LAST_EXIT_CODE".into(), Value::int(0, Span::unknown()));

    let mut line_editor = get_line_editor(engine_state, nushell_path, use_color)?;
    let temp_file = temp_dir().join(format!("{}.nu", uuid::Uuid::new_v4()));

    if let Some(s) = prerun_command {
        eval_source(
            engine_state,
            &mut unique_stack,
            s.item.as_bytes(),
            &format!("entry #{entry_num}"),
            PipelineData::empty(),
            false,
        );
        let cwd = get_guaranteed_cwd(engine_state, &unique_stack);
        engine_state.merge_env(&mut unique_stack, cwd)?;
    }

    let hostname = System::host_name();
    if shell_integration {
        shell_integration_osc_7_633_2(hostname.as_deref(), engine_state, &mut unique_stack);
    }

    engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);

    // Regenerate the $nu constant to contain the startup time and any other potential updates
    let nu_const = create_nu_constant(engine_state, Span::unknown())?;
    engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);

    if load_std_lib.is_none() && engine_state.get_config().show_banner {
        eval_source(
            engine_state,
            &mut unique_stack,
            r#"use std banner; banner"#.as_bytes(),
            "show_banner",
            PipelineData::empty(),
            false,
        );
    }

    kitty_protocol_healthcheck(engine_state);

    // Setup initial engine_state and stack state
    let mut previous_engine_state = engine_state.clone();
    let mut previous_stack_arc = Arc::new(unique_stack);
    loop {
        // clone these values so that they can be moved by AssertUnwindSafe
        // If there is a panic within this iteration the last engine_state and stack
        // will be used
        let mut current_engine_state = previous_engine_state.clone();
        // for the stack, we are going to hold to create a child stack instead,
        // avoiding an expensive copy
        let current_stack = Stack::with_parent(previous_stack_arc.clone());
        let temp_file_cloned = temp_file.clone();
        let mut nu_prompt_cloned = nu_prompt.clone();

        let iteration_panic_state = catch_unwind(AssertUnwindSafe(|| {
            let (continue_loop, current_stack, line_editor) = loop_iteration(LoopContext {
                engine_state: &mut current_engine_state,
                stack: current_stack,
                line_editor,
                nu_prompt: &mut nu_prompt_cloned,
                temp_file: &temp_file_cloned,
                use_color,
                entry_num: &mut entry_num,
                hostname: hostname.as_deref(),
            });

            // pass the most recent version of the line_editor back
            (
                continue_loop,
                current_engine_state,
                current_stack,
                line_editor,
            )
        }));
        match iteration_panic_state {
            Ok((continue_loop, es, s, le)) => {
                // setup state for the next iteration of the repl loop
                previous_engine_state = es;
                // we apply the changes from the updated stack back onto our previous stack
                previous_stack_arc =
                    Arc::new(Stack::with_changes_from_child(previous_stack_arc, s));
                line_editor = le;
                if !continue_loop {
                    break;
                }
            }
            Err(_) => {
                // line_editor is lost in the error case so reconstruct a new one
                line_editor = get_line_editor(engine_state, nushell_path, use_color)?;
            }
        }
    }

    Ok(())
}

fn get_line_editor(
    engine_state: &mut EngineState,
    nushell_path: &str,
    use_color: bool,
) -> Result<Reedline> {
    let mut start_time = std::time::Instant::now();
    let mut line_editor = Reedline::create();

    // Now that reedline is created, get the history session id and store it in engine_state
    store_history_id_in_engine(engine_state, &line_editor);
    perf(
        "setup reedline",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    if let Some(history) = engine_state.history_config() {
        start_time = std::time::Instant::now();

        line_editor = setup_history(nushell_path, engine_state, line_editor, history)?;

        perf(
            "setup history",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }
    Ok(line_editor)
}

struct LoopContext<'a> {
    engine_state: &'a mut EngineState,
    stack: Stack,
    line_editor: Reedline,
    nu_prompt: &'a mut NushellPrompt,
    temp_file: &'a Path,
    use_color: bool,
    entry_num: &'a mut usize,
    hostname: Option<&'a str>,
}

/// Perform one iteration of the REPL loop
/// Result is bool: continue loop, current reedline
#[inline]
fn loop_iteration(ctx: LoopContext) -> (bool, Stack, Reedline) {
    use nu_cmd_base::hook;
    use reedline::Signal;
    let loop_start_time = std::time::Instant::now();

    let LoopContext {
        engine_state,
        mut stack,
        line_editor,
        nu_prompt,
        temp_file,
        use_color,
        entry_num,
        hostname,
    } = ctx;

    let cwd = get_guaranteed_cwd(engine_state, &stack);

    let mut start_time = std::time::Instant::now();
    // Before doing anything, merge the environment from the previous REPL iteration into the
    // permanent state.
    if let Err(err) = engine_state.merge_env(&mut stack, cwd) {
        report_error_new(engine_state, &err);
    }
    perf(
        "merge env",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // Reset the ctrl-c handler
    if let Some(ctrlc) = &mut engine_state.ctrlc {
        ctrlc.store(false, Ordering::SeqCst);
    }
    perf(
        "reset ctrlc",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // Right before we start our prompt and take input from the user,
    // fire the "pre_prompt" hook
    if let Some(hook) = engine_state.get_config().hooks.pre_prompt.clone() {
        if let Err(err) = eval_hook(engine_state, &mut stack, None, vec![], &hook, "pre_prompt") {
            report_error_new(engine_state, &err);
        }
    }
    perf(
        "pre-prompt hook",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // Next, check all the environment variables they ask for
    // fire the "env_change" hook
    let env_change = engine_state.get_config().hooks.env_change.clone();
    if let Err(error) = hook::eval_env_change_hook(env_change, engine_state, &mut stack) {
        report_error_new(engine_state, &error)
    }
    perf(
        "env-change hook",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let engine_reference = Arc::new(engine_state.clone());
    let config = engine_state.get_config();

    start_time = std::time::Instant::now();
    // Find the configured cursor shapes for each mode
    let cursor_config = CursorConfig {
        vi_insert: map_nucursorshape_to_cursorshape(config.cursor_shape_vi_insert),
        vi_normal: map_nucursorshape_to_cursorshape(config.cursor_shape_vi_normal),
        emacs: map_nucursorshape_to_cursorshape(config.cursor_shape_emacs),
    };
    perf(
        "get config/cursor config",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // at this line we have cloned the state for the completer and the transient prompt
    // until we drop those, we cannot use the stack in the REPL loop itself
    // See STACK-REFERENCE to see where we have taken a reference
    let stack_arc = Arc::new(stack);

    let mut line_editor = line_editor
        .use_kitty_keyboard_enhancement(config.use_kitty_protocol)
        // try to enable bracketed paste
        // It doesn't work on windows system: https://github.com/crossterm-rs/crossterm/issues/737
        .use_bracketed_paste(cfg!(not(target_os = "windows")) && config.bracketed_paste)
        .with_highlighter(Box::new(NuHighlighter {
            engine_state: engine_reference.clone(),
            // STACK-REFERENCE 1
            stack: stack_arc.clone(),
            config: config.clone(),
        }))
        .with_validator(Box::new(NuValidator {
            engine_state: engine_reference.clone(),
        }))
        .with_completer(Box::new(NuCompleter::new(
            engine_reference.clone(),
            // STACK-REFERENCE 2
            Stack::with_parent(stack_arc.clone()),
        )))
        .with_quick_completions(config.quick_completions)
        .with_partial_completions(config.partial_completions)
        .with_ansi_colors(config.use_ansi_coloring)
        .with_cursor_config(cursor_config);

    perf(
        "reedline builder",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let style_computer = StyleComputer::from_config(engine_state, &stack_arc);

    start_time = std::time::Instant::now();
    line_editor = if config.use_ansi_coloring {
        line_editor.with_hinter(Box::new({
            // As of Nov 2022, "hints" color_config closures only get `null` passed in.
            let style = style_computer.compute("hints", &Value::nothing(Span::unknown()));
            CwdAwareHinter::default().with_style(style)
        }))
    } else {
        line_editor.disable_hints()
    };

    perf(
        "reedline coloring/style_computer",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    trace!("adding menus");
    line_editor =
        add_menus(line_editor, engine_reference, &stack_arc, config).unwrap_or_else(|e| {
            report_error_new(engine_state, &e);
            Reedline::create()
        });

    perf(
        "reedline adding menus",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    let buffer_editor = get_editor(engine_state, &stack_arc, Span::unknown());

    line_editor = if let Ok((cmd, args)) = buffer_editor {
        let mut command = std::process::Command::new(cmd);
        let envs = env_to_strings(engine_state, &stack_arc).unwrap_or_else(|e| {
            warn!("Couldn't convert environment variable values to strings: {e}");
            HashMap::default()
        });
        command.args(args).envs(envs);
        line_editor.with_buffer_editor(command, temp_file.to_path_buf())
    } else {
        line_editor
    };

    perf(
        "reedline buffer_editor",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    if let Some(history) = engine_state.history_config() {
        start_time = std::time::Instant::now();
        if history.sync_on_enter {
            if let Err(e) = line_editor.sync_history() {
                warn!("Failed to sync history: {}", e);
            }
        }

        perf(
            "sync_history",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }

    start_time = std::time::Instant::now();
    // Changing the line editor based on the found keybindings
    line_editor = setup_keybindings(engine_state, line_editor);

    perf(
        "keybindings",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    let config = &engine_state.get_config().clone();
    prompt_update::update_prompt(
        config,
        engine_state,
        &mut Stack::with_parent(stack_arc.clone()),
        nu_prompt,
    );
    let transient_prompt = prompt_update::make_transient_prompt(
        config,
        engine_state,
        &mut Stack::with_parent(stack_arc.clone()),
        nu_prompt,
    );

    perf(
        "update_prompt",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    *entry_num += 1;

    start_time = std::time::Instant::now();
    line_editor = line_editor.with_transient_prompt(transient_prompt);
    let input = line_editor.read_line(nu_prompt);
    // we got our inputs, we can now drop our stack references
    // This lists all of the stack references that we have cleaned up
    line_editor = line_editor
        // CLEAR STACK-REFERENCE 1
        .with_highlighter(Box::<NoOpHighlighter>::default())
        // CLEAR STACK-REFERENCE 2
        .with_completer(Box::<DefaultCompleter>::default());
    let shell_integration = config.shell_integration;

    let mut stack = Stack::unwrap_unique(stack_arc);

    perf(
        "line_editor setup",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let line_editor_input_time = std::time::Instant::now();
    match input {
        Ok(Signal::Success(s)) => {
            let history_supports_meta = matches!(
                engine_state.history_config().map(|h| h.file_format),
                Some(HistoryFileFormat::Sqlite)
            );

            if history_supports_meta {
                prepare_history_metadata(&s, hostname, engine_state, &mut line_editor);
            }

            // For pre_exec_hook
            start_time = Instant::now();

            // Right before we start running the code the user gave us, fire the `pre_execution`
            // hook
            if let Some(hook) = config.hooks.pre_execution.clone() {
                // Set the REPL buffer to the current command for the "pre_execution" hook
                let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
                repl.buffer = s.to_string();
                drop(repl);

                if let Err(err) = eval_hook(
                    engine_state,
                    &mut stack,
                    None,
                    vec![],
                    &hook,
                    "pre_execution",
                ) {
                    report_error_new(engine_state, &err);
                }
            }

            perf(
                "pre_execution_hook",
                start_time,
                file!(),
                line!(),
                column!(),
                use_color,
            );

            let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
            repl.cursor_pos = line_editor.current_insertion_point();
            repl.buffer = line_editor.current_buffer_contents().to_string();
            drop(repl);

            if shell_integration {
                start_time = Instant::now();

                run_ansi_sequence(PRE_EXECUTE_MARKER);

                perf(
                    "pre_execute_marker (133;C) ansi escape sequence",
                    start_time,
                    file!(),
                    line!(),
                    column!(),
                    use_color,
                );
            }

            // Actual command execution logic starts from here
            let cmd_execution_start_time = Instant::now();

            match parse_operation(s.clone(), engine_state, &stack) {
                Ok(operation) => match operation {
                    ReplOperation::AutoCd { cwd, target, span } => {
                        do_auto_cd(target, cwd, &mut stack, engine_state, span);

                        if shell_integration {
                            start_time = Instant::now();

                            run_ansi_sequence(&get_command_finished_marker(&stack, engine_state));

                            perf(
                                "post_execute_marker (133;D) ansi escape sequences",
                                start_time,
                                file!(),
                                line!(),
                                column!(),
                                use_color,
                            );
                        }
                    }
                    ReplOperation::RunCommand(cmd) => {
                        line_editor = do_run_cmd(
                            &cmd,
                            &mut stack,
                            engine_state,
                            line_editor,
                            shell_integration,
                            *entry_num,
                            use_color,
                        );

                        if shell_integration {
                            start_time = Instant::now();

                            run_ansi_sequence(&get_command_finished_marker(&stack, engine_state));

                            perf(
                                "post_execute_marker (133;D) ansi escape sequences",
                                start_time,
                                file!(),
                                line!(),
                                column!(),
                                use_color,
                            );
                        }
                    }
                    // as the name implies, we do nothing in this case
                    ReplOperation::DoNothing => {}
                },
                Err(ref e) => error!("Error parsing operation: {e}"),
            }
            let cmd_duration = cmd_execution_start_time.elapsed();

            stack.add_env_var(
                "CMD_DURATION_MS".into(),
                Value::string(format!("{}", cmd_duration.as_millis()), Span::unknown()),
            );

            if history_supports_meta {
                if let Err(e) = fill_in_result_related_history_metadata(
                    &s,
                    engine_state,
                    cmd_duration,
                    &mut stack,
                    &mut line_editor,
                ) {
                    warn!("Could not fill in result related history metadata: {e}");
                }
            }

            if shell_integration {
                start_time = Instant::now();

                shell_integration_osc_7_633_2(hostname, engine_state, &mut stack);

                perf(
                    "shell_integration_finalize ansi escape sequences",
                    start_time,
                    file!(),
                    line!(),
                    column!(),
                    use_color,
                );
            }

            flush_engine_state_repl_buffer(engine_state, &mut line_editor);
        }
        Ok(Signal::CtrlC) => {
            // `Reedline` clears the line content. New prompt is shown
            if shell_integration {
                start_time = Instant::now();

                run_ansi_sequence(&get_command_finished_marker(&stack, engine_state));

                perf(
                    "command_finished_marker ansi escape sequence",
                    start_time,
                    file!(),
                    line!(),
                    column!(),
                    use_color,
                );
            }
        }
        Ok(Signal::CtrlD) => {
            // When exiting clear to a new line
            if shell_integration {
                start_time = Instant::now();

                run_ansi_sequence(&get_command_finished_marker(&stack, engine_state));

                perf(
                    "command_finished_marker ansi escape sequence",
                    start_time,
                    file!(),
                    line!(),
                    column!(),
                    use_color,
                );
            }
            println!();
            return (false, stack, line_editor);
        }
        Err(err) => {
            let message = err.to_string();
            if !message.contains("duration") {
                eprintln!("Error: {err:?}");
                // TODO: Identify possible error cases where a hard failure is preferable
                // Ignoring and reporting could hide bigger problems
                // e.g. https://github.com/nushell/nushell/issues/6452
                // Alternatively only allow that expected failures let the REPL loop
            }
            if shell_integration {
                start_time = Instant::now();

                run_ansi_sequence(&get_command_finished_marker(&stack, engine_state));

                perf(
                    "command_finished_marker ansi escape sequence",
                    start_time,
                    file!(),
                    line!(),
                    column!(),
                    use_color,
                );
            }
        }
    }
    perf(
        "processing line editor input",
        line_editor_input_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    perf(
        "time between prompts in line editor loop",
        loop_start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    (true, stack, line_editor)
}

///
/// Put in history metadata not related to the result of running the command
///
fn prepare_history_metadata(
    s: &str,
    hostname: Option<&str>,
    engine_state: &EngineState,
    line_editor: &mut Reedline,
) {
    if !s.is_empty() && line_editor.has_last_command_context() {
        let result = line_editor
            .update_last_command_context(&|mut c| {
                c.start_timestamp = Some(chrono::Utc::now());
                c.hostname = hostname.map(str::to_string);

                c.cwd = Some(StateWorkingSet::new(engine_state).get_cwd());
                c
            })
            .into_diagnostic();
        if let Err(e) = result {
            warn!("Could not prepare history metadata: {e}");
        }
    }
}

///
/// Fills in history item metadata based on the execution result (notably duration and exit code)
///
fn fill_in_result_related_history_metadata(
    s: &str,
    engine_state: &EngineState,
    cmd_duration: Duration,
    stack: &mut Stack,
    line_editor: &mut Reedline,
) -> Result<()> {
    if !s.is_empty() && line_editor.has_last_command_context() {
        line_editor
            .update_last_command_context(&|mut c| {
                c.duration = Some(cmd_duration);
                c.exit_status = stack
                    .get_env_var(engine_state, "LAST_EXIT_CODE")
                    .and_then(|e| e.as_i64().ok());
                c
            })
            .into_diagnostic()?; // todo: don't stop repl if error here?
    }
    Ok(())
}

/// The kinds of operations you can do in a single loop iteration of the REPL
enum ReplOperation {
    /// "auto-cd": change directory by typing it in directly
    AutoCd {
        /// the current working directory
        cwd: String,
        /// the target
        target: PathBuf,
        /// span information for debugging
        span: Span,
    },
    /// run a command
    RunCommand(String),
    /// do nothing (usually through an empty string)
    DoNothing,
}

///
/// Parses one "REPL line" of input, to try and derive intent.
/// Notably, this is where we detect whether the user is attempting an
/// "auto-cd" (writing a relative path directly instead of `cd path`)
///
/// Returns the ReplOperation we believe the user wants to do
///
fn parse_operation(
    s: String,
    engine_state: &EngineState,
    stack: &Stack,
) -> Result<ReplOperation, ErrReport> {
    let tokens = lex(s.as_bytes(), 0, &[], &[], false);
    // Check if this is a single call to a directory, if so auto-cd
    let cwd = nu_engine::env::current_dir_str(engine_state, stack)?;
    let mut orig = s.clone();
    if orig.starts_with('`') {
        orig = trim_quotes_str(&orig).to_string()
    }

    let path = nu_path::expand_path_with(&orig, &cwd, true);
    if looks_like_path(&orig) && path.is_dir() && tokens.0.len() == 1 {
        Ok(ReplOperation::AutoCd {
            cwd,
            target: path,
            span: tokens.0[0].span,
        })
    } else if !s.trim().is_empty() {
        Ok(ReplOperation::RunCommand(s))
    } else {
        Ok(ReplOperation::DoNothing)
    }
}

///
/// Execute an "auto-cd" operation, changing the current working directory.
///
fn do_auto_cd(
    path: PathBuf,
    cwd: String,
    stack: &mut Stack,
    engine_state: &mut EngineState,
    span: Span,
) {
    let path = {
        if !path.exists() {
            report_error_new(
                engine_state,
                &ShellError::DirectoryNotFound {
                    dir: path.to_string_lossy().to_string(),
                    span,
                },
            );
        }
        let path = nu_path::canonicalize_with(path, &cwd)
            .expect("internal error: cannot canonicalize known path");
        path.to_string_lossy().to_string()
    };

    if let PermissionResult::PermissionDenied(reason) = have_permission(path.clone()) {
        report_error_new(
            engine_state,
            &ShellError::IOError {
                msg: format!("Cannot change directory to {path}: {reason}"),
            },
        );
        return;
    }

    stack.add_env_var("OLDPWD".into(), Value::string(cwd.clone(), Span::unknown()));

    //FIXME: this only changes the current scope, but instead this environment variable
    //should probably be a block that loads the information from the state in the overlay
    stack.add_env_var("PWD".into(), Value::string(path.clone(), Span::unknown()));
    let cwd = Value::string(cwd, span);

    let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
    let mut shells = if let Some(v) = shells {
        v.into_list().unwrap_or_else(|_| vec![cwd])
    } else {
        vec![cwd]
    };

    let current_shell = stack.get_env_var(engine_state, "NUSHELL_CURRENT_SHELL");
    let current_shell = if let Some(v) = current_shell {
        v.as_int().unwrap_or_default() as usize
    } else {
        0
    };

    let last_shell = stack.get_env_var(engine_state, "NUSHELL_LAST_SHELL");
    let last_shell = if let Some(v) = last_shell {
        v.as_int().unwrap_or_default() as usize
    } else {
        0
    };

    shells[current_shell] = Value::string(path, span);

    stack.add_env_var("NUSHELL_SHELLS".into(), Value::list(shells, span));
    stack.add_env_var(
        "NUSHELL_LAST_SHELL".into(),
        Value::int(last_shell as i64, span),
    );
    stack.add_env_var("LAST_EXIT_CODE".into(), Value::int(0, Span::unknown()));
}

///
/// Run a command as received from reedline. This is where we are actually
/// running a thing!
///
fn do_run_cmd(
    s: &str,
    stack: &mut Stack,
    engine_state: &mut EngineState,
    // we pass in the line editor so it can be dropped in the case of a process exit
    // (in the normal case we don't want to drop it so return it as-is otherwise)
    line_editor: Reedline,
    shell_integration: bool,
    entry_num: usize,
    use_color: bool,
) -> Reedline {
    trace!("eval source: {}", s);

    let mut cmds = s.split_whitespace();
    if let Some("exit") = cmds.next() {
        let mut working_set = StateWorkingSet::new(engine_state);
        let _ = parse(&mut working_set, None, s.as_bytes(), false);

        if working_set.parse_errors.is_empty() {
            match cmds.next() {
                Some(s) => {
                    if let Ok(n) = s.parse::<i32>() {
                        drop(line_editor);
                        std::process::exit(n);
                    }
                }
                None => {
                    drop(line_editor);
                    std::process::exit(0);
                }
            }
        }
    }

    if shell_integration {
        let start_time = Instant::now();
        if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
            match cwd.coerce_into_string() {
                Ok(path) => {
                    // Try to abbreviate string for windows title
                    let maybe_abbrev_path = if let Some(p) = nu_path::home_dir() {
                        path.replace(&p.as_path().display().to_string(), "~")
                    } else {
                        path
                    };
                    let binary_name = s.split_whitespace().next();

                    if let Some(binary_name) = binary_name {
                        run_ansi_sequence(&format!(
                            "\x1b]2;{maybe_abbrev_path}> {binary_name}\x07"
                        ));
                    }
                }
                Err(e) => {
                    warn!("Could not coerce working directory to string {e}");
                }
            }
        }

        perf(
            "set title with command ansi escape sequence",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }

    eval_source(
        engine_state,
        stack,
        s.as_bytes(),
        &format!("entry #{entry_num}"),
        PipelineData::empty(),
        false,
    );

    line_editor
}

///
/// Output some things and set environment variables so shells with the right integration
/// can have more information about what is going on (both on startup and after we have
/// run a command)
///
fn shell_integration_osc_7_633_2(
    hostname: Option<&str>,
    engine_state: &EngineState,
    stack: &mut Stack,
) {
    if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
        match cwd.coerce_into_string() {
            Ok(path) => {
                // Supported escape sequences of Microsoft's Visual Studio Code (vscode)
                // https://code.visualstudio.com/docs/terminal/shell-integration#_supported-escape-sequences
                if stack.get_env_var(engine_state, "TERM_PROGRAM")
                    == Some(Value::test_string("vscode"))
                {
                    // If we're in vscode, run their specific ansi escape sequence.
                    // This is helpful for ctrl+g to change directories in the terminal.
                    run_ansi_sequence(&format!("\x1b]633;P;Cwd={}\x1b\\", path));
                } else {
                    // Otherwise, communicate the path as OSC 7 (often used for spawning new tabs in the same dir)
                    run_ansi_sequence(&format!(
                        "\x1b]7;file://{}{}{}\x1b\\",
                        percent_encoding::utf8_percent_encode(
                            hostname.unwrap_or("localhost"),
                            percent_encoding::CONTROLS
                        ),
                        if path.starts_with('/') { "" } else { "/" },
                        percent_encoding::utf8_percent_encode(&path, percent_encoding::CONTROLS)
                    ));
                }

                // Try to abbreviate string for windows title
                let maybe_abbrev_path = if let Some(p) = nu_path::home_dir() {
                    path.replace(&p.as_path().display().to_string(), "~")
                } else {
                    path
                };

                // Set window title too
                // https://tldp.org/HOWTO/Xterm-Title-3.html
                // ESC]0;stringBEL -- Set icon name and window title to string
                // ESC]1;stringBEL -- Set icon name to string
                // ESC]2;stringBEL -- Set window title to string
                run_ansi_sequence(&format!("\x1b]2;{maybe_abbrev_path}\x07"));
            }
            Err(e) => {
                warn!("Could not coerce working directory to string {e}");
            }
        }
    }
    run_ansi_sequence(RESET_APPLICATION_MODE);
}

///
/// Clear the screen and output anything remaining in the EngineState buffer.
///
fn flush_engine_state_repl_buffer(engine_state: &mut EngineState, line_editor: &mut Reedline) {
    let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
    line_editor.run_edit_commands(&[
        EditCommand::Clear,
        EditCommand::InsertString(repl.buffer.to_string()),
        EditCommand::MoveToPosition {
            position: repl.cursor_pos,
            select: false,
        },
    ]);
    repl.buffer = "".to_string();
    repl.cursor_pos = 0;
}

///
/// Setup history management for Reedline
///
fn setup_history(
    nushell_path: &str,
    engine_state: &mut EngineState,
    line_editor: Reedline,
    history: HistoryConfig,
) -> Result<Reedline> {
    // Setup history_isolation aka "history per session"
    let history_session_id = if history.isolation {
        Reedline::create_history_session_id()
    } else {
        None
    };

    if let Some(path) = crate::config_files::get_history_path(nushell_path, history.file_format) {
        return update_line_editor_history(
            engine_state,
            path,
            history,
            line_editor,
            history_session_id,
        );
    };
    Ok(line_editor)
}

///
/// Setup Reedline keybindingds based on the provided config
///
fn setup_keybindings(engine_state: &EngineState, line_editor: Reedline) -> Reedline {
    return match create_keybindings(engine_state.get_config()) {
        Ok(keybindings) => match keybindings {
            KeybindingsMode::Emacs(keybindings) => {
                let edit_mode = Box::new(Emacs::new(keybindings));
                line_editor.with_edit_mode(edit_mode)
            }
            KeybindingsMode::Vi {
                insert_keybindings,
                normal_keybindings,
            } => {
                let edit_mode = Box::new(Vi::new(insert_keybindings, normal_keybindings));
                line_editor.with_edit_mode(edit_mode)
            }
        },
        Err(e) => {
            report_error_new(engine_state, &e);
            line_editor
        }
    };
}

///
/// Make sure that the terminal supports the kitty protocol if the config is asking for it
///
fn kitty_protocol_healthcheck(engine_state: &EngineState) {
    if engine_state.get_config().use_kitty_protocol && !reedline::kitty_protocol_available() {
        warn!("Terminal doesn't support use_kitty_protocol config");
    }
}

fn store_history_id_in_engine(engine_state: &mut EngineState, line_editor: &Reedline) {
    let session_id = line_editor
        .get_history_session_id()
        .map(i64::from)
        .unwrap_or(0);

    engine_state.history_session_id = session_id;
}

fn update_line_editor_history(
    engine_state: &mut EngineState,
    history_path: PathBuf,
    history: HistoryConfig,
    line_editor: Reedline,
    history_session_id: Option<HistorySessionId>,
) -> Result<Reedline, ErrReport> {
    let history: Box<dyn reedline::History> = match history.file_format {
        HistoryFileFormat::PlainText => Box::new(
            FileBackedHistory::with_file(history.max_size as usize, history_path)
                .into_diagnostic()?,
        ),
        HistoryFileFormat::Sqlite => Box::new(
            SqliteBackedHistory::with_file(
                history_path.to_path_buf(),
                history_session_id,
                Some(chrono::Utc::now()),
            )
            .into_diagnostic()?,
        ),
    };
    let line_editor = line_editor
        .with_history_session_id(history_session_id)
        .with_history_exclusion_prefix(Some(" ".into()))
        .with_history(history);

    store_history_id_in_engine(engine_state, &line_editor);

    Ok(line_editor)
}

fn confirm_stdin_is_terminal() -> Result<()> {
    // Guard against invocation without a connected terminal.
    // reedline / crossterm event polling will fail without a connected tty
    if !std::io::stdin().is_terminal() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Nushell launched as a REPL, but STDIN is not a TTY; either launch in a valid terminal or provide arguments to invoke a script!",
        ))
        .into_diagnostic();
    }
    Ok(())
}
fn map_nucursorshape_to_cursorshape(shape: NuCursorShape) -> Option<SetCursorStyle> {
    match shape {
        NuCursorShape::Block => Some(SetCursorStyle::SteadyBlock),
        NuCursorShape::UnderScore => Some(SetCursorStyle::SteadyUnderScore),
        NuCursorShape::Line => Some(SetCursorStyle::SteadyBar),
        NuCursorShape::BlinkBlock => Some(SetCursorStyle::BlinkingBlock),
        NuCursorShape::BlinkUnderScore => Some(SetCursorStyle::BlinkingUnderScore),
        NuCursorShape::BlinkLine => Some(SetCursorStyle::BlinkingBar),
        NuCursorShape::Inherit => None,
    }
}

fn get_command_finished_marker(stack: &Stack, engine_state: &EngineState) -> String {
    let exit_code = stack
        .get_env_var(engine_state, "LAST_EXIT_CODE")
        .and_then(|e| e.as_i64().ok());

    format!("\x1b]133;D;{}\x1b\\", exit_code.unwrap_or(0))
}

fn run_ansi_sequence(seq: &str) {
    if let Err(e) = io::stdout().write_all(seq.as_bytes()) {
        warn!("Error writing ansi sequence {e}");
    } else if let Err(e) = io::stdout().flush() {
        warn!("Error flushing stdio {e}");
    }
}

// Absolute paths with a drive letter, like 'C:', 'D:\', 'E:\foo'
#[cfg(windows)]
static DRIVE_PATH_REGEX: once_cell::sync::Lazy<fancy_regex::Regex> =
    once_cell::sync::Lazy::new(|| {
        fancy_regex::Regex::new(r"^[a-zA-Z]:[/\\]?").expect("Internal error: regex creation")
    });

// A best-effort "does this string look kinda like a path?" function to determine whether to auto-cd
fn looks_like_path(orig: &str) -> bool {
    #[cfg(windows)]
    {
        if DRIVE_PATH_REGEX.is_match(orig).unwrap_or(false) {
            return true;
        }
    }

    orig.starts_with('.')
        || orig.starts_with('~')
        || orig.starts_with('/')
        || orig.starts_with('\\')
        || orig.ends_with(std::path::MAIN_SEPARATOR)
}

#[cfg(windows)]
#[test]
fn looks_like_path_windows_drive_path_works() {
    assert!(looks_like_path("C:"));
    assert!(looks_like_path("D:\\"));
    assert!(looks_like_path("E:/"));
    assert!(looks_like_path("F:\\some_dir"));
    assert!(looks_like_path("G:/some_dir"));
}

#[cfg(windows)]
#[test]
fn trailing_slash_looks_like_path() {
    assert!(looks_like_path("foo\\"))
}

#[cfg(not(windows))]
#[test]
fn trailing_slash_looks_like_path() {
    assert!(looks_like_path("foo/"))
}

#[test]
fn are_session_ids_in_sync() {
    let engine_state = &mut EngineState::new();
    let history = engine_state.history_config().unwrap();
    let history_path =
        crate::config_files::get_history_path("nushell", history.file_format).unwrap();
    let line_editor = reedline::Reedline::create();
    let history_session_id = reedline::Reedline::create_history_session_id();
    let line_editor = update_line_editor_history(
        engine_state,
        history_path,
        history,
        line_editor,
        history_session_id,
    );
    assert_eq!(
        i64::from(line_editor.unwrap().get_history_session_id().unwrap()),
        engine_state.history_session_id
    );
}
