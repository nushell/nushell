use crate::prompt_update::{
    POST_EXECUTION_MARKER_PREFIX, POST_EXECUTION_MARKER_SUFFIX, PRE_EXECUTION_MARKER,
    RESET_APPLICATION_MODE, VSCODE_COMMANDLINE_MARKER_PREFIX, VSCODE_COMMANDLINE_MARKER_SUFFIX,
    VSCODE_CWD_PROPERTY_MARKER_PREFIX, VSCODE_CWD_PROPERTY_MARKER_SUFFIX,
    VSCODE_POST_EXECUTION_MARKER_PREFIX, VSCODE_POST_EXECUTION_MARKER_SUFFIX,
    VSCODE_PRE_EXECUTION_MARKER,
};
use crate::{
    NuHighlighter, NuValidator, NushellPrompt,
    completions::NuCompleter,
    nu_highlight::NoOpHighlighter,
    prompt_update,
    reedline_config::{KeybindingsMode, add_menus, create_keybindings},
    util::eval_source,
};
use crossterm::cursor::SetCursorStyle;
use log::{error, trace, warn};
use miette::{ErrReport, IntoDiagnostic, Result};
use nu_cmd_base::util::get_editor;
use nu_color_config::StyleComputer;
#[allow(deprecated)]
use nu_engine::env_to_strings;
use nu_engine::exit::cleanup_exit;
use nu_parser::{lex, parse, trim_quotes_str};
use nu_protocol::shell_error::io::IoError;
use nu_protocol::{BannerKind, shell_error};
use nu_protocol::{
    HistoryConfig, HistoryFileFormat, PipelineData, ShellError, Span, Spanned, Value,
    config::NuCursorShape,
    engine::{EngineState, Stack, StateWorkingSet},
    report_shell_error,
};
use nu_utils::{
    filesystem::{PermissionResult, have_permission},
    perf,
};
use reedline::{
    CursorConfig, CwdAwareHinter, DefaultCompleter, EditCommand, Emacs, FileBackedHistory,
    HistorySessionId, Reedline, SqliteBackedHistory, Vi,
};
use std::sync::atomic::Ordering;
use std::{
    collections::HashMap,
    env::temp_dir,
    io::{self, IsTerminal, Write},
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use sysinfo::System;

/// The main REPL loop, including spinning up the prompt itself.
pub fn evaluate_repl(
    engine_state: &mut EngineState,
    stack: Stack,
    prerun_command: Option<Spanned<String>>,
    load_std_lib: Option<Spanned<String>>,
    entire_start_time: Instant,
) -> Result<()> {
    // throughout this code, we hold this stack uniquely.
    // During the main REPL loop, we hand ownership of this value to an Arc,
    // so that it may be read by various reedline plugins. During this, we
    // can't modify the stack, but at the end of the loop we take back ownership
    // from the Arc. This lets us avoid copying stack variables needlessly
    let mut unique_stack = stack.clone();
    let config = engine_state.get_config();
    let use_color = config.use_ansi_coloring.get(engine_state);

    let mut entry_num = 0;

    // Let's grab the shell_integration configs
    let shell_integration_osc2 = config.shell_integration.osc2;
    let shell_integration_osc7 = config.shell_integration.osc7;
    let shell_integration_osc9_9 = config.shell_integration.osc9_9;
    let shell_integration_osc133 = config.shell_integration.osc133;
    let shell_integration_osc633 = config.shell_integration.osc633;

    let nu_prompt = NushellPrompt::new(
        shell_integration_osc133,
        shell_integration_osc633,
        engine_state.clone(),
        stack.clone(),
    );

    // seed env vars
    unique_stack.add_env_var(
        "CMD_DURATION_MS".into(),
        Value::string("0823", Span::unknown()),
    );

    unique_stack.set_last_exit_code(0, Span::unknown());

    let mut line_editor = get_line_editor(engine_state, use_color)?;
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
        engine_state.merge_env(&mut unique_stack)?;
    }

    confirm_stdin_is_terminal()?;

    let hostname = System::host_name();
    if shell_integration_osc2 {
        run_shell_integration_osc2(None, engine_state, &mut unique_stack, use_color);
    }
    if shell_integration_osc7 {
        run_shell_integration_osc7(
            hostname.as_deref(),
            engine_state,
            &mut unique_stack,
            use_color,
        );
    }
    if shell_integration_osc9_9 {
        run_shell_integration_osc9_9(engine_state, &mut unique_stack, use_color);
    }
    if shell_integration_osc633 {
        // escape a few things because this says so
        // https://code.visualstudio.com/docs/terminal/shell-integration#_vs-code-custom-sequences-osc-633-st
        let cmd_text = line_editor.current_buffer_contents().to_string();

        let replaced_cmd_text = escape_special_vscode_bytes(&cmd_text)?;

        run_shell_integration_osc633(
            engine_state,
            &mut unique_stack,
            use_color,
            replaced_cmd_text,
        );
    }

    engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);

    // Regenerate the $nu constant to contain the startup time and any other potential updates
    engine_state.generate_nu_constant();

    if load_std_lib.is_none() {
        match engine_state.get_config().show_banner {
            BannerKind::None => {}
            BannerKind::Short => {
                eval_source(
                    engine_state,
                    &mut unique_stack,
                    r#"banner --short"#.as_bytes(),
                    "show short banner",
                    PipelineData::empty(),
                    false,
                );
            }
            BannerKind::Full => {
                eval_source(
                    engine_state,
                    &mut unique_stack,
                    r#"banner"#.as_bytes(),
                    "show_banner",
                    PipelineData::empty(),
                    false,
                );
            }
        }
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
                line_editor = get_line_editor(engine_state, use_color)?;
            }
        }
    }

    Ok(())
}

fn escape_special_vscode_bytes(input: &str) -> Result<String, ShellError> {
    let bytes = input
        .chars()
        .flat_map(|c| {
            let mut buf = [0; 4]; // Buffer to hold UTF-8 bytes of the character
            let c_bytes = c.encode_utf8(&mut buf); // Get UTF-8 bytes for the character

            if c_bytes.len() == 1 {
                let byte = c_bytes.as_bytes()[0];

                match byte {
                    // Escape bytes below 0x20
                    b if b < 0x20 => format!("\\x{byte:02X}").into_bytes(),
                    // Escape semicolon as \x3B
                    b';' => "\\x3B".to_string().into_bytes(),
                    // Escape backslash as \\
                    b'\\' => "\\\\".to_string().into_bytes(),
                    // Otherwise, return the character unchanged
                    _ => vec![byte],
                }
            } else {
                // pass through multi-byte characters unchanged
                c_bytes.bytes().collect()
            }
        })
        .collect();

    String::from_utf8(bytes).map_err(|err| ShellError::CantConvert {
        to_type: "string".to_string(),
        from_type: "bytes".to_string(),
        span: Span::unknown(),
        help: Some(format!(
            "Error {err}, Unable to convert {input} to escaped bytes"
        )),
    })
}

fn get_line_editor(engine_state: &mut EngineState, use_color: bool) -> Result<Reedline> {
    let mut start_time = std::time::Instant::now();
    let mut line_editor = Reedline::create();

    // Now that reedline is created, get the history session id and store it in engine_state
    store_history_id_in_engine(engine_state, &line_editor);
    perf!("setup reedline", start_time, use_color);

    if let Some(history) = engine_state.history_config() {
        start_time = std::time::Instant::now();

        line_editor = setup_history(engine_state, line_editor, history)?;

        perf!("setup history", start_time, use_color);
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

    let mut start_time = std::time::Instant::now();
    // Before doing anything, merge the environment from the previous REPL iteration into the
    // permanent state.
    if let Err(err) = engine_state.merge_env(&mut stack) {
        report_shell_error(engine_state, &err);
    }
    perf!("merge env", start_time, use_color);

    start_time = std::time::Instant::now();
    engine_state.reset_signals();
    perf!("reset signals", start_time, use_color);

    start_time = std::time::Instant::now();
    // Check all the environment variables they ask for
    // fire the "env_change" hook
    if let Err(error) = hook::eval_env_change_hook(
        &engine_state.get_config().hooks.env_change.clone(),
        engine_state,
        &mut stack,
    ) {
        report_shell_error(engine_state, &error)
    }
    perf!("env-change hook", start_time, use_color);

    start_time = std::time::Instant::now();
    // Next, right before we start our prompt and take input from the user, fire the "pre_prompt" hook
    if let Err(err) = hook::eval_hooks(
        engine_state,
        &mut stack,
        vec![],
        &engine_state.get_config().hooks.pre_prompt.clone(),
        "pre_prompt",
    ) {
        report_shell_error(engine_state, &err);
    }
    perf!("pre-prompt hook", start_time, use_color);

    let engine_reference = Arc::new(engine_state.clone());
    let config = stack.get_config(engine_state);

    start_time = std::time::Instant::now();
    // Find the configured cursor shapes for each mode
    let cursor_config = CursorConfig {
        vi_insert: map_nucursorshape_to_cursorshape(config.cursor_shape.vi_insert),
        vi_normal: map_nucursorshape_to_cursorshape(config.cursor_shape.vi_normal),
        emacs: map_nucursorshape_to_cursorshape(config.cursor_shape.emacs),
    };
    perf!("get config/cursor config", start_time, use_color);

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
        }))
        .with_validator(Box::new(NuValidator {
            engine_state: engine_reference.clone(),
        }))
        .with_completer(Box::new(NuCompleter::new(
            engine_reference.clone(),
            // STACK-REFERENCE 2
            stack_arc.clone(),
        )))
        .with_quick_completions(config.completions.quick)
        .with_partial_completions(config.completions.partial)
        .with_ansi_colors(config.use_ansi_coloring.get(engine_state))
        .with_cwd(Some(
            engine_state
                .cwd(None)
                .map(|cwd| cwd.into_std_path_buf())
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        ))
        .with_cursor_config(cursor_config)
        .with_visual_selection_style(nu_ansi_term::Style {
            is_reverse: true,
            ..Default::default()
        });

    perf!("reedline builder", start_time, use_color);

    let style_computer = StyleComputer::from_config(engine_state, &stack_arc);

    start_time = std::time::Instant::now();
    line_editor = if config.use_ansi_coloring.get(engine_state) {
        line_editor.with_hinter(Box::new({
            // As of Nov 2022, "hints" color_config closures only get `null` passed in.
            let style = style_computer.compute("hints", &Value::nothing(Span::unknown()));
            CwdAwareHinter::default().with_style(style)
        }))
    } else {
        line_editor.disable_hints()
    };

    perf!("reedline coloring/style_computer", start_time, use_color);

    start_time = std::time::Instant::now();
    trace!("adding menus");
    line_editor =
        add_menus(line_editor, engine_reference, &stack_arc, config).unwrap_or_else(|e| {
            report_shell_error(engine_state, &e);
            Reedline::create()
        });

    perf!("reedline adding menus", start_time, use_color);

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

    perf!("reedline buffer_editor", start_time, use_color);

    if let Some(history) = engine_state.history_config() {
        start_time = std::time::Instant::now();
        if history.sync_on_enter {
            if let Err(e) = line_editor.sync_history() {
                warn!("Failed to sync history: {e}");
            }
        }

        perf!("sync_history", start_time, use_color);
    }

    start_time = std::time::Instant::now();
    // Changing the line editor based on the found keybindings
    line_editor = setup_keybindings(engine_state, line_editor);

    perf!("keybindings", start_time, use_color);

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

    perf!("update_prompt", start_time, use_color);

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
        .with_completer(Box::<DefaultCompleter>::default())
        // Ensure immediately accept is always cleared
        .with_immediately_accept(false);

    // Let's grab the shell_integration configs
    let shell_integration_osc2 = config.shell_integration.osc2;
    let shell_integration_osc7 = config.shell_integration.osc7;
    let shell_integration_osc9_9 = config.shell_integration.osc9_9;
    let shell_integration_osc133 = config.shell_integration.osc133;
    let shell_integration_osc633 = config.shell_integration.osc633;
    let shell_integration_reset_application_mode = config.shell_integration.reset_application_mode;

    // TODO: we may clone the stack, this can lead to major performance issues
    // so we should avoid it or making stack cheaper to clone.
    let mut stack = Arc::unwrap_or_clone(stack_arc);

    perf!("line_editor setup", start_time, use_color);

    let line_editor_input_time = std::time::Instant::now();
    match input {
        Ok(Signal::Success(repl_cmd_line_text)) => {
            let history_supports_meta = matches!(
                engine_state.history_config().map(|h| h.file_format),
                Some(HistoryFileFormat::Sqlite)
            );

            if history_supports_meta {
                prepare_history_metadata(
                    &repl_cmd_line_text,
                    hostname,
                    engine_state,
                    &mut line_editor,
                );
            }

            // For pre_exec_hook
            start_time = Instant::now();

            // Right before we start running the code the user gave us, fire the `pre_execution`
            // hook
            {
                // Set the REPL buffer to the current command for the "pre_execution" hook
                let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
                repl.buffer = repl_cmd_line_text.to_string();
                drop(repl);

                if let Err(err) = hook::eval_hooks(
                    engine_state,
                    &mut stack,
                    vec![],
                    &engine_state.get_config().hooks.pre_execution.clone(),
                    "pre_execution",
                ) {
                    report_shell_error(engine_state, &err);
                }
            }

            perf!("pre_execution_hook", start_time, use_color);

            let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
            repl.cursor_pos = line_editor.current_insertion_point();
            repl.buffer = line_editor.current_buffer_contents().to_string();
            drop(repl);

            if shell_integration_osc633 {
                if stack
                    .get_env_var(engine_state, "TERM_PROGRAM")
                    .and_then(|v| v.as_str().ok())
                    == Some("vscode")
                {
                    start_time = Instant::now();

                    run_ansi_sequence(VSCODE_PRE_EXECUTION_MARKER);

                    perf!(
                        "pre_execute_marker (633;C) ansi escape sequence",
                        start_time,
                        use_color
                    );
                } else if shell_integration_osc133 {
                    start_time = Instant::now();

                    run_ansi_sequence(PRE_EXECUTION_MARKER);

                    perf!(
                        "pre_execute_marker (133;C) ansi escape sequence",
                        start_time,
                        use_color
                    );
                }
            } else if shell_integration_osc133 {
                start_time = Instant::now();

                run_ansi_sequence(PRE_EXECUTION_MARKER);

                perf!(
                    "pre_execute_marker (133;C) ansi escape sequence",
                    start_time,
                    use_color
                );
            }

            // Actual command execution logic starts from here
            let cmd_execution_start_time = Instant::now();

            match parse_operation(repl_cmd_line_text.clone(), engine_state, &stack) {
                Ok(operation) => match operation {
                    ReplOperation::AutoCd { cwd, target, span } => {
                        do_auto_cd(target, cwd, &mut stack, engine_state, span);

                        run_finaliziation_ansi_sequence(
                            &stack,
                            engine_state,
                            use_color,
                            shell_integration_osc633,
                            shell_integration_osc133,
                        );
                    }
                    ReplOperation::RunCommand(cmd) => {
                        line_editor = do_run_cmd(
                            &cmd,
                            &mut stack,
                            engine_state,
                            line_editor,
                            shell_integration_osc2,
                            *entry_num,
                            use_color,
                        );

                        run_finaliziation_ansi_sequence(
                            &stack,
                            engine_state,
                            use_color,
                            shell_integration_osc633,
                            shell_integration_osc133,
                        );
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
                    &repl_cmd_line_text,
                    engine_state,
                    cmd_duration,
                    &mut stack,
                    &mut line_editor,
                ) {
                    warn!("Could not fill in result related history metadata: {e}");
                }
            }

            if shell_integration_osc2 {
                run_shell_integration_osc2(None, engine_state, &mut stack, use_color);
            }
            if shell_integration_osc7 {
                run_shell_integration_osc7(hostname, engine_state, &mut stack, use_color);
            }
            if shell_integration_osc9_9 {
                run_shell_integration_osc9_9(engine_state, &mut stack, use_color);
            }
            if shell_integration_osc633 {
                run_shell_integration_osc633(
                    engine_state,
                    &mut stack,
                    use_color,
                    repl_cmd_line_text,
                );
            }
            if shell_integration_reset_application_mode {
                run_shell_integration_reset_application_mode();
            }

            line_editor = flush_engine_state_repl_buffer(engine_state, line_editor);
        }
        Ok(Signal::CtrlC) => {
            // `Reedline` clears the line content. New prompt is shown
            run_finaliziation_ansi_sequence(
                &stack,
                engine_state,
                use_color,
                shell_integration_osc633,
                shell_integration_osc133,
            );
        }
        Ok(Signal::CtrlD) => {
            // When exiting clear to a new line

            run_finaliziation_ansi_sequence(
                &stack,
                engine_state,
                use_color,
                shell_integration_osc633,
                shell_integration_osc133,
            );

            println!();

            cleanup_exit((), engine_state, 0);

            // if cleanup_exit didn't exit, we should keep running
            return (true, stack, line_editor);
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

            run_finaliziation_ansi_sequence(
                &stack,
                engine_state,
                use_color,
                shell_integration_osc633,
                shell_integration_osc133,
            );
        }
    }
    perf!(
        "processing line editor input",
        line_editor_input_time,
        use_color
    );

    perf!(
        "time between prompts in line editor loop",
        loop_start_time,
        use_color
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
                c.cwd = engine_state
                    .cwd(None)
                    .ok()
                    .map(|path| path.to_string_lossy().to_string());
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
                    .and_then(|e| e.as_int().ok());
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
    let cwd = engine_state
        .cwd(Some(stack))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
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
            report_shell_error(
                engine_state,
                &ShellError::Io(IoError::new_with_additional_context(
                    shell_error::io::ErrorKind::DirectoryNotFound,
                    span,
                    PathBuf::from(&path),
                    "Cannot change directory",
                )),
            );
        }
        path.to_string_lossy().to_string()
    };

    if let PermissionResult::PermissionDenied = have_permission(path.clone()) {
        report_shell_error(
            engine_state,
            &ShellError::Io(IoError::new_with_additional_context(
                shell_error::io::ErrorKind::from_std(std::io::ErrorKind::PermissionDenied),
                span,
                PathBuf::from(path),
                "Cannot change directory",
            )),
        );
        return;
    }

    stack.add_env_var("OLDPWD".into(), Value::string(cwd.clone(), Span::unknown()));

    //FIXME: this only changes the current scope, but instead this environment variable
    //should probably be a block that loads the information from the state in the overlay
    if let Err(err) = stack.set_cwd(&path) {
        report_shell_error(engine_state, &err);
        return;
    };
    let cwd = Value::string(cwd, span);

    let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
    let mut shells = if let Some(v) = shells {
        v.clone().into_list().unwrap_or_else(|_| vec![cwd])
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
    stack.set_last_exit_code(0, Span::unknown());
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
    shell_integration_osc2: bool,
    entry_num: usize,
    use_color: bool,
) -> Reedline {
    trace!("eval source: {s}");

    let mut cmds = s.split_whitespace();

    let had_warning_before = engine_state.exit_warning_given.load(Ordering::SeqCst);

    if let Some("exit") = cmds.next() {
        let mut working_set = StateWorkingSet::new(engine_state);
        let _ = parse(&mut working_set, None, s.as_bytes(), false);

        if working_set.parse_errors.is_empty() {
            match cmds.next() {
                Some(s) => {
                    if let Ok(n) = s.parse::<i32>() {
                        return cleanup_exit(line_editor, engine_state, n);
                    }
                }
                None => {
                    return cleanup_exit(line_editor, engine_state, 0);
                }
            }
        }
    }

    if shell_integration_osc2 {
        run_shell_integration_osc2(Some(s), engine_state, stack, use_color);
    }

    eval_source(
        engine_state,
        stack,
        s.as_bytes(),
        &format!("entry #{entry_num}"),
        PipelineData::empty(),
        false,
    );

    // if there was a warning before, and we got to this point, it means
    // the possible call to cleanup_exit did not occur.
    if had_warning_before && engine_state.is_interactive {
        engine_state
            .exit_warning_given
            .store(false, Ordering::SeqCst);
    }

    line_editor
}

///
/// Output some things and set environment variables so shells with the right integration
/// can have more information about what is going on (both on startup and after we have
/// run a command)
///
fn run_shell_integration_osc2(
    command_name: Option<&str>,
    engine_state: &EngineState,
    stack: &mut Stack,
    use_color: bool,
) {
    if let Ok(path) = engine_state.cwd_as_string(Some(stack)) {
        let start_time = Instant::now();

        // Try to abbreviate string for windows title
        let maybe_abbrev_path = if let Some(p) = nu_path::home_dir() {
            let home_dir_str = p.as_path().display().to_string();
            if path.starts_with(&home_dir_str) {
                path.replacen(&home_dir_str, "~", 1)
            } else {
                path
            }
        } else {
            path
        };

        let title = match command_name {
            Some(binary_name) => {
                let split_binary_name = binary_name.split_whitespace().next();
                if let Some(binary_name) = split_binary_name {
                    format!("{maybe_abbrev_path}> {binary_name}")
                } else {
                    maybe_abbrev_path.to_string()
                }
            }
            None => maybe_abbrev_path.to_string(),
        };

        // Set window title too
        // https://tldp.org/HOWTO/Xterm-Title-3.html
        // ESC]0;stringBEL -- Set icon name and window title to string
        // ESC]1;stringBEL -- Set icon name to string
        // ESC]2;stringBEL -- Set window title to string
        run_ansi_sequence(&format!("\x1b]2;{title}\x07"));

        perf!("set title with command osc2", start_time, use_color);
    }
}

fn run_shell_integration_osc7(
    hostname: Option<&str>,
    engine_state: &EngineState,
    stack: &mut Stack,
    use_color: bool,
) {
    if let Ok(path) = engine_state.cwd_as_string(Some(stack)) {
        let start_time = Instant::now();

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

        perf!(
            "communicate path to terminal with osc7",
            start_time,
            use_color
        );
    }
}

fn run_shell_integration_osc9_9(engine_state: &EngineState, stack: &mut Stack, use_color: bool) {
    if let Ok(path) = engine_state.cwd_as_string(Some(stack)) {
        let start_time = Instant::now();

        // Otherwise, communicate the path as OSC 9;9 from ConEmu (often used for spawning new tabs in the same dir)
        // This is helpful in Windows Terminal with Duplicate Tab
        run_ansi_sequence(&format!(
            "\x1b]9;9;{}\x1b\\",
            percent_encoding::utf8_percent_encode(&path, percent_encoding::CONTROLS)
        ));

        perf!(
            "communicate path to terminal with osc9;9",
            start_time,
            use_color
        );
    }
}

fn run_shell_integration_osc633(
    engine_state: &EngineState,
    stack: &mut Stack,
    use_color: bool,
    repl_cmd_line_text: String,
) {
    if let Ok(path) = engine_state.cwd_as_string(Some(stack)) {
        // Supported escape sequences of Microsoft's Visual Studio Code (vscode)
        // https://code.visualstudio.com/docs/terminal/shell-integration#_supported-escape-sequences
        if stack
            .get_env_var(engine_state, "TERM_PROGRAM")
            .and_then(|v| v.as_str().ok())
            == Some("vscode")
        {
            let start_time = Instant::now();

            // If we're in vscode, run their specific ansi escape sequence.
            // This is helpful for ctrl+g to change directories in the terminal.
            run_ansi_sequence(&format!(
                "{VSCODE_CWD_PROPERTY_MARKER_PREFIX}{path}{VSCODE_CWD_PROPERTY_MARKER_SUFFIX}"
            ));

            perf!(
                "communicate path to terminal with osc633;P",
                start_time,
                use_color
            );

            // escape a few things because this says so
            // https://code.visualstudio.com/docs/terminal/shell-integration#_vs-code-custom-sequences-osc-633-st
            let replaced_cmd_text =
                escape_special_vscode_bytes(&repl_cmd_line_text).unwrap_or(repl_cmd_line_text);

            //OSC 633 ; E ; <commandline> [; <nonce] ST - Explicitly set the command line with an optional nonce.
            run_ansi_sequence(&format!(
                "{VSCODE_COMMANDLINE_MARKER_PREFIX}{replaced_cmd_text}{VSCODE_COMMANDLINE_MARKER_SUFFIX}"
            ));
        }
    }
}

fn run_shell_integration_reset_application_mode() {
    run_ansi_sequence(RESET_APPLICATION_MODE);
}

///
/// Clear the screen and output anything remaining in the EngineState buffer.
///
fn flush_engine_state_repl_buffer(
    engine_state: &mut EngineState,
    mut line_editor: Reedline,
) -> Reedline {
    let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
    line_editor.run_edit_commands(&[
        EditCommand::Clear,
        EditCommand::InsertString(repl.buffer.to_string()),
        EditCommand::MoveToPosition {
            position: repl.cursor_pos,
            select: false,
        },
    ]);
    if repl.accept {
        line_editor = line_editor.with_immediately_accept(true)
    }
    repl.accept = false;
    repl.buffer = "".to_string();
    repl.cursor_pos = 0;
    line_editor
}

///
/// Setup history management for Reedline
///
fn setup_history(
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

    if let Some(path) = history.file_path() {
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
    match create_keybindings(engine_state.get_config()) {
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
            report_shell_error(engine_state, &e);
            line_editor
        }
    }
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
        HistoryFileFormat::Plaintext => Box::new(
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
        NuCursorShape::Underscore => Some(SetCursorStyle::SteadyUnderScore),
        NuCursorShape::Line => Some(SetCursorStyle::SteadyBar),
        NuCursorShape::BlinkBlock => Some(SetCursorStyle::BlinkingBlock),
        NuCursorShape::BlinkUnderscore => Some(SetCursorStyle::BlinkingUnderScore),
        NuCursorShape::BlinkLine => Some(SetCursorStyle::BlinkingBar),
        NuCursorShape::Inherit => None,
    }
}

fn get_command_finished_marker(
    stack: &Stack,
    engine_state: &EngineState,
    shell_integration_osc633: bool,
    shell_integration_osc133: bool,
) -> String {
    let exit_code = stack
        .get_env_var(engine_state, "LAST_EXIT_CODE")
        .and_then(|e| e.as_int().ok());

    if shell_integration_osc633 {
        if stack
            .get_env_var(engine_state, "TERM_PROGRAM")
            .and_then(|v| v.as_str().ok())
            == Some("vscode")
        {
            // We're in vscode and we have osc633 enabled
            format!(
                "{}{}{}",
                VSCODE_POST_EXECUTION_MARKER_PREFIX,
                exit_code.unwrap_or(0),
                VSCODE_POST_EXECUTION_MARKER_SUFFIX
            )
        } else if shell_integration_osc133 {
            // If we're in VSCode but we don't find the env var, just return the regular markers
            format!(
                "{}{}{}",
                POST_EXECUTION_MARKER_PREFIX,
                exit_code.unwrap_or(0),
                POST_EXECUTION_MARKER_SUFFIX
            )
        } else {
            // We're not in vscode, so we don't need to do anything special
            "\x1b[0m".to_string()
        }
    } else if shell_integration_osc133 {
        format!(
            "{}{}{}",
            POST_EXECUTION_MARKER_PREFIX,
            exit_code.unwrap_or(0),
            POST_EXECUTION_MARKER_SUFFIX
        )
    } else {
        "\x1b[0m".to_string()
    }
}

fn run_ansi_sequence(seq: &str) {
    if let Err(e) = io::stdout().write_all(seq.as_bytes()) {
        warn!("Error writing ansi sequence {e}");
    } else if let Err(e) = io::stdout().flush() {
        warn!("Error flushing stdio {e}");
    }
}

fn run_finaliziation_ansi_sequence(
    stack: &Stack,
    engine_state: &EngineState,
    use_color: bool,
    shell_integration_osc633: bool,
    shell_integration_osc133: bool,
) {
    if shell_integration_osc633 {
        // Only run osc633 if we are in vscode
        if stack
            .get_env_var(engine_state, "TERM_PROGRAM")
            .and_then(|v| v.as_str().ok())
            == Some("vscode")
        {
            let start_time = Instant::now();

            run_ansi_sequence(&get_command_finished_marker(
                stack,
                engine_state,
                shell_integration_osc633,
                shell_integration_osc133,
            ));

            perf!(
                "post_execute_marker (633;D) ansi escape sequences",
                start_time,
                use_color
            );
        } else if shell_integration_osc133 {
            let start_time = Instant::now();

            run_ansi_sequence(&get_command_finished_marker(
                stack,
                engine_state,
                shell_integration_osc633,
                shell_integration_osc133,
            ));

            perf!(
                "post_execute_marker (133;D) ansi escape sequences",
                start_time,
                use_color
            );
        }
    } else if shell_integration_osc133 {
        let start_time = Instant::now();

        run_ansi_sequence(&get_command_finished_marker(
            stack,
            engine_state,
            shell_integration_osc633,
            shell_integration_osc133,
        ));

        perf!(
            "post_execute_marker (133;D) ansi escape sequences",
            start_time,
            use_color
        );
    }
}

// Absolute paths with a drive letter, like 'C:', 'D:\', 'E:\foo'
#[cfg(windows)]
static DRIVE_PATH_REGEX: std::sync::LazyLock<fancy_regex::Regex> = std::sync::LazyLock::new(|| {
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
    let history_path = history.file_path().unwrap();
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

#[cfg(test)]
mod test_auto_cd {
    use super::{ReplOperation, do_auto_cd, escape_special_vscode_bytes, parse_operation};
    use nu_path::AbsolutePath;
    use nu_protocol::engine::{EngineState, Stack};
    use tempfile::tempdir;

    /// Create a symlink. Works on both Unix and Windows.
    #[cfg(any(unix, windows))]
    fn symlink(
        original: impl AsRef<AbsolutePath>,
        link: impl AsRef<AbsolutePath>,
    ) -> std::io::Result<()> {
        let original = original.as_ref();
        let link = link.as_ref();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original, link)
        }
        #[cfg(windows)]
        {
            if original.is_dir() {
                std::os::windows::fs::symlink_dir(original, link)
            } else {
                std::os::windows::fs::symlink_file(original, link)
            }
        }
    }

    /// Run one test case on the auto-cd feature. PWD is initially set to
    /// `before`, and after `input` is parsed and evaluated, PWD should be
    /// changed to `after`.
    #[track_caller]
    fn check(before: impl AsRef<AbsolutePath>, input: &str, after: impl AsRef<AbsolutePath>) {
        // Setup EngineState and Stack.
        let mut engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.set_cwd(before.as_ref()).unwrap();

        // Parse the input. It must be an auto-cd operation.
        let op = parse_operation(input.to_string(), &engine_state, &stack).unwrap();
        let ReplOperation::AutoCd { cwd, target, span } = op else {
            panic!("'{input}' was not parsed into an auto-cd operation")
        };

        // Perform the auto-cd operation.
        do_auto_cd(target, cwd, &mut stack, &mut engine_state, span);
        let updated_cwd = engine_state.cwd(Some(&stack)).unwrap();

        // Check that `updated_cwd` and `after` point to the same place. They
        // don't have to be byte-wise equal (on Windows, the 8.3 filename
        // conversion messes things up),
        let updated_cwd = std::fs::canonicalize(updated_cwd).unwrap();
        let after = std::fs::canonicalize(after.as_ref()).unwrap();
        assert_eq!(updated_cwd, after);
    }

    #[test]
    fn auto_cd_root() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let input = if cfg!(windows) { r"C:\" } else { "/" };
        let root = AbsolutePath::try_new(input).unwrap();
        check(tempdir, input, root);
    }

    #[test]
    fn auto_cd_tilde() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let home = nu_path::home_dir().unwrap();
        check(tempdir, "~", home);
    }

    #[test]
    fn auto_cd_dot() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        check(tempdir, ".", tempdir);
    }

    #[test]
    fn auto_cd_double_dot() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let dir = tempdir.join("foo");
        std::fs::create_dir_all(&dir).unwrap();
        check(dir, "..", tempdir);
    }

    #[test]
    fn auto_cd_triple_dot() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let dir = tempdir.join("foo").join("bar");
        std::fs::create_dir_all(&dir).unwrap();
        check(dir, "...", tempdir);
    }

    #[test]
    fn auto_cd_relative() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let foo = tempdir.join("foo");
        let bar = tempdir.join("bar");
        std::fs::create_dir_all(&foo).unwrap();
        std::fs::create_dir_all(&bar).unwrap();
        let input = if cfg!(windows) { r"..\bar" } else { "../bar" };
        check(foo, input, bar);
    }

    #[test]
    fn auto_cd_trailing_slash() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let dir = tempdir.join("foo");
        std::fs::create_dir_all(&dir).unwrap();
        let input = if cfg!(windows) { r"foo\" } else { "foo/" };
        check(tempdir, input, dir);
    }

    #[test]
    fn auto_cd_symlink() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let dir = tempdir.join("foo");
        std::fs::create_dir_all(&dir).unwrap();
        let link = tempdir.join("link");
        symlink(&dir, &link).unwrap();
        let input = if cfg!(windows) { r".\link" } else { "./link" };
        check(tempdir, input, link);

        let dir = tempdir.join("foo").join("bar");
        std::fs::create_dir_all(&dir).unwrap();
        let link = tempdir.join("link2");
        symlink(&dir, &link).unwrap();
        let input = "..";
        check(link, input, tempdir);
    }

    #[test]
    #[should_panic(expected = "was not parsed into an auto-cd operation")]
    fn auto_cd_nonexistent_directory() {
        let tempdir = tempdir().unwrap();
        let tempdir = AbsolutePath::try_new(tempdir.path()).unwrap();

        let dir = tempdir.join("foo");
        let input = if cfg!(windows) { r"foo\" } else { "foo/" };
        check(tempdir, input, dir);
    }

    #[test]
    fn escape_vscode_semicolon_test() {
        let input = r#"now;is"#;
        let expected = r#"now\x3Bis"#;
        let actual = escape_special_vscode_bytes(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn escape_vscode_backslash_test() {
        let input = r#"now\is"#;
        let expected = r#"now\\is"#;
        let actual = escape_special_vscode_bytes(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn escape_vscode_linefeed_test() {
        let input = "now\nis";
        let expected = r#"now\x0Ais"#;
        let actual = escape_special_vscode_bytes(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn escape_vscode_tab_null_cr_test() {
        let input = "now\t\0\ris";
        let expected = r#"now\x09\x00\x0Dis"#;
        let actual = escape_special_vscode_bytes(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn escape_vscode_multibyte_ok() {
        let input = "now🍪is";
        let actual = escape_special_vscode_bytes(input).unwrap();
        assert_eq!(input, actual);
    }
}
