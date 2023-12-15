use crate::{
    completions::NuCompleter,
    prompt_update,
    reedline_config::{add_menus, create_keybindings, KeybindingsMode},
    util::eval_source,
    NuHighlighter, NuValidator, NushellPrompt,
};
use crossterm::cursor::SetCursorStyle;
use log::{trace, warn};
use miette::{ErrReport, IntoDiagnostic, Result};
use nu_cmd_base::util::get_guaranteed_cwd;
use nu_cmd_base::{hook::eval_hook, util::get_editor};
use nu_color_config::StyleComputer;
use nu_engine::convert_env_values;
use nu_parser::{lex, parse, trim_quotes_str};
use nu_protocol::{
    config::NuCursorShape,
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::create_nu_constant,
    report_error, report_error_new, HistoryFileFormat, PipelineData, ShellError, Span, Spanned,
    Value, NU_VARIABLE_ID,
};
use nu_utils::utils::perf;
use reedline::{
    CursorConfig, CwdAwareHinter, EditCommand, Emacs, FileBackedHistory, HistorySessionId,
    Reedline, SqliteBackedHistory, Vi,
};
use std::{
    env::temp_dir,
    io::{self, IsTerminal, Write},
    path::Path,
    sync::atomic::Ordering,
    time::Instant,
};
use sysinfo::SystemExt;

// According to Daniel Imms @Tyriar, we need to do these this way:
// <133 A><prompt><133 B><command><133 C><command output>
// These first two have been moved to prompt_update to get as close as possible to the prompt.
// const PRE_PROMPT_MARKER: &str = "\x1b]133;A\x1b\\";
// const POST_PROMPT_MARKER: &str = "\x1b]133;B\x1b\\";
const PRE_EXECUTE_MARKER: &str = "\x1b]133;C\x1b\\";
// This one is in get_command_finished_marker() now so we can capture the exit codes properly.
// const CMD_FINISHED_MARKER: &str = "\x1b]133;D;{}\x1b\\";
const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

pub fn evaluate_repl(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    nushell_path: &str,
    prerun_command: Option<Spanned<String>>,
    load_std_lib: Option<Spanned<String>>,
    entire_start_time: Instant,
) -> Result<()> {
    use nu_cmd_base::hook;
    use reedline::Signal;
    let use_color = engine_state.get_config().use_ansi_coloring;

    // Guard against invocation without a connected terminal.
    // reedline / crossterm event polling will fail without a connected tty
    if !std::io::stdin().is_terminal() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Nushell launched as a REPL, but STDIN is not a TTY; either launch in a valid terminal or provide arguments to invoke a script!",
        ))
        .into_diagnostic();
    }

    let mut entry_num = 0;

    let mut nu_prompt = NushellPrompt::new();

    let start_time = std::time::Instant::now();
    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
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
    stack.add_env_var(
        "CMD_DURATION_MS".into(),
        Value::string("0823", Span::unknown()),
    );

    stack.add_env_var("LAST_EXIT_CODE".into(), Value::int(0, Span::unknown()));

    let mut start_time = std::time::Instant::now();
    let mut line_editor = Reedline::create();
    let temp_file = temp_dir().join(format!("{}.nu", uuid::Uuid::new_v4()));

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

    // Setup history_isolation aka "history per session"
    let history_isolation = engine_state.get_config().history_isolation;
    let history_session_id = if history_isolation {
        Reedline::create_history_session_id()
    } else {
        None
    };

    start_time = std::time::Instant::now();
    let history_path = crate::config_files::get_history_path(
        nushell_path,
        engine_state.config.history_file_format,
    );
    if let Some(history_path) = history_path.as_deref() {
        line_editor =
            update_line_editor_history(engine_state, history_path, line_editor, history_session_id)?
    };
    perf(
        "setup history",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    let sys = sysinfo::System::new();
    perf(
        "get sysinfo",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    if let Some(s) = prerun_command {
        eval_source(
            engine_state,
            stack,
            s.item.as_bytes(),
            &format!("entry #{entry_num}"),
            PipelineData::empty(),
            false,
        );
        engine_state.merge_env(stack, get_guaranteed_cwd(engine_state, stack))?;
    }

    engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);

    // Regenerate the $nu constant to contain the startup time and any other potential updates
    let nu_const = create_nu_constant(engine_state, Span::unknown())?;
    engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);

    if load_std_lib.is_none() && engine_state.get_config().show_banner {
        eval_source(
            engine_state,
            stack,
            r#"use std banner; banner"#.as_bytes(),
            "show_banner",
            PipelineData::empty(),
            false,
        );
    }

    if engine_state.get_config().use_kitty_protocol && !reedline::kitty_protocol_available() {
        warn!("Terminal doesn't support use_kitty_protocol config");
    }

    loop {
        let loop_start_time = std::time::Instant::now();

        let cwd = get_guaranteed_cwd(engine_state, stack);

        start_time = std::time::Instant::now();
        // Before doing anything, merge the environment from the previous REPL iteration into the
        // permanent state.
        if let Err(err) = engine_state.merge_env(stack, cwd) {
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
        //Reset the ctrl-c handler
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
        // Reset the SIGQUIT handler
        if let Some(sig_quit) = engine_state.get_sig_quit() {
            sig_quit.store(false, Ordering::SeqCst);
        }
        perf(
            "reset sig_quit",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        start_time = std::time::Instant::now();
        let config = engine_state.get_config();

        let engine_reference = std::sync::Arc::new(engine_state.clone());

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

        line_editor = line_editor
            .use_kitty_keyboard_enhancement(config.use_kitty_protocol)
            // try to enable bracketed paste
            // It doesn't work on windows system: https://github.com/crossterm-rs/crossterm/issues/737
            .use_bracketed_paste(cfg!(not(target_os = "windows")) && config.bracketed_paste)
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_reference.clone(),
                config: config.clone(),
            }))
            .with_validator(Box::new(NuValidator {
                engine_state: engine_reference.clone(),
            }))
            .with_completer(Box::new(NuCompleter::new(
                engine_reference.clone(),
                stack.clone(),
            )))
            .with_quick_completions(config.quick_completions)
            .with_partial_completions(config.partial_completions)
            .with_ansi_colors(config.use_ansi_coloring)
            .with_cursor_config(cursor_config)
            .with_transient_prompt(prompt_update::transient_prompt(
                engine_reference.clone(),
                stack,
            ));
        perf(
            "reedline builder",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        let style_computer = StyleComputer::from_config(engine_state, stack);

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
        line_editor = add_menus(line_editor, engine_reference, stack, config).unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
            Reedline::create()
        });
        perf(
            "reedline menus",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        start_time = std::time::Instant::now();
        let buffer_editor = get_editor(engine_state, stack, Span::unknown());

        line_editor = if let Ok((cmd, args)) = buffer_editor {
            let mut command = std::process::Command::new(&cmd);
            command.args(args).envs(
                engine_state
                    .render_env_vars()
                    .into_iter()
                    .filter_map(|(k, v)| v.as_string().ok().map(|v| (k, v))),
            );
            line_editor.with_buffer_editor(command, temp_file.clone())
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

        start_time = std::time::Instant::now();
        if config.sync_history_on_enter {
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

        start_time = std::time::Instant::now();
        // Changing the line editor based on the found keybindings
        line_editor = match create_keybindings(config) {
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
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
                line_editor
            }
        };
        perf(
            "keybindings",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        start_time = std::time::Instant::now();
        // Right before we start our prompt and take input from the user,
        // fire the "pre_prompt" hook
        if let Some(hook) = config.hooks.pre_prompt.clone() {
            if let Err(err) = eval_hook(engine_state, stack, None, vec![], &hook, "pre_prompt") {
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
        let config = engine_state.get_config();
        if let Err(error) =
            hook::eval_env_change_hook(config.hooks.env_change.clone(), engine_state, stack)
        {
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

        start_time = std::time::Instant::now();
        let config = &engine_state.get_config().clone();
        let prompt = prompt_update::update_prompt(config, engine_state, stack, &mut nu_prompt);
        perf(
            "update_prompt",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        entry_num += 1;

        start_time = std::time::Instant::now();
        let input = line_editor.read_line(prompt);
        let shell_integration = config.shell_integration;

        match input {
            Ok(Signal::Success(s)) => {
                let hostname = sys.host_name();
                let history_supports_meta =
                    matches!(config.history_file_format, HistoryFileFormat::Sqlite);
                if history_supports_meta && !s.is_empty() && line_editor.has_last_command_context()
                {
                    line_editor
                        .update_last_command_context(&|mut c| {
                            c.start_timestamp = Some(chrono::Utc::now());
                            c.hostname = hostname.clone();

                            c.cwd = Some(StateWorkingSet::new(engine_state).get_cwd());
                            c
                        })
                        .into_diagnostic()?; // todo: don't stop repl if error here?
                }

                // Right before we start running the code the user gave us, fire the `pre_execution`
                // hook
                if let Some(hook) = config.hooks.pre_execution.clone() {
                    // Set the REPL buffer to the current command for the "pre_execution" hook
                    let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
                    repl.buffer = s.to_string();
                    drop(repl);

                    if let Err(err) =
                        eval_hook(engine_state, stack, None, vec![], &hook, "pre_execution")
                    {
                        report_error_new(engine_state, &err);
                    }
                }

                let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
                repl.cursor_pos = line_editor.current_insertion_point();
                repl.buffer = line_editor.current_buffer_contents().to_string();
                drop(repl);

                if shell_integration {
                    run_ansi_sequence(PRE_EXECUTE_MARKER)?;
                }

                let start_time = Instant::now();
                let tokens = lex(s.as_bytes(), 0, &[], &[], false);
                // Check if this is a single call to a directory, if so auto-cd
                let cwd = nu_engine::env::current_dir_str(engine_state, stack)?;

                let mut orig = s.clone();
                if orig.starts_with('`') {
                    orig = trim_quotes_str(&orig).to_string()
                }

                let path = nu_path::expand_path_with(&orig, &cwd);

                if looks_like_path(&orig) && path.is_dir() && tokens.0.len() == 1 {
                    // We have an auto-cd
                    let (path, span) = {
                        if !path.exists() {
                            let working_set = StateWorkingSet::new(engine_state);

                            report_error(
                                &working_set,
                                &ShellError::DirectoryNotFound {
                                    dir: path.to_string_lossy().to_string(),
                                    span: tokens.0[0].span,
                                },
                            );
                        }
                        let path = nu_path::canonicalize_with(path, &cwd)
                            .expect("internal error: cannot canonicalize known path");
                        (path.to_string_lossy().to_string(), tokens.0[0].span)
                    };

                    stack.add_env_var("OLDPWD".into(), Value::string(cwd.clone(), Span::unknown()));

                    //FIXME: this only changes the current scope, but instead this environment variable
                    //should probably be a block that loads the information from the state in the overlay
                    stack.add_env_var("PWD".into(), Value::string(path.clone(), Span::unknown()));
                    let cwd = Value::string(cwd, span);

                    let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
                    let mut shells = if let Some(v) = shells {
                        v.as_list()
                            .map(|x| x.to_vec())
                            .unwrap_or_else(|_| vec![cwd])
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
                } else if !s.trim().is_empty() {
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

                    eval_source(
                        engine_state,
                        stack,
                        s.as_bytes(),
                        &format!("entry #{entry_num}"),
                        PipelineData::empty(),
                        false,
                    );
                }
                let cmd_duration = start_time.elapsed();

                stack.add_env_var(
                    "CMD_DURATION_MS".into(),
                    Value::string(format!("{}", cmd_duration.as_millis()), Span::unknown()),
                );

                if history_supports_meta && !s.is_empty() && line_editor.has_last_command_context()
                {
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

                if shell_integration {
                    run_ansi_sequence(&get_command_finished_marker(stack, engine_state))?;
                    if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
                        let path = cwd.as_string()?;

                        // Supported escape sequences of Microsoft's Visual Studio Code (vscode)
                        // https://code.visualstudio.com/docs/terminal/shell-integration#_supported-escape-sequences
                        if stack.get_env_var(engine_state, "TERM_PROGRAM")
                            == Some(Value::test_string("vscode"))
                        {
                            // If we're in vscode, run their specific ansi escape sequence.
                            // This is helpful for ctrl+g to change directories in the terminal.
                            run_ansi_sequence(&format!("\x1b]633;P;Cwd={}\x1b\\", path))?;
                        } else {
                            // Otherwise, communicate the path as OSC 7 (often used for spawning new tabs in the same dir)
                            run_ansi_sequence(&format!(
                                "\x1b]7;file://{}{}{}\x1b\\",
                                percent_encoding::utf8_percent_encode(
                                    &hostname.unwrap_or_else(|| "localhost".to_string()),
                                    percent_encoding::CONTROLS
                                ),
                                if path.starts_with('/') { "" } else { "/" },
                                percent_encoding::utf8_percent_encode(
                                    &path,
                                    percent_encoding::CONTROLS
                                )
                            ))?;
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
                        run_ansi_sequence(&format!("\x1b]2;{maybe_abbrev_path}\x07"))?;
                    }
                    run_ansi_sequence(RESET_APPLICATION_MODE)?;
                }

                let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
                line_editor.run_edit_commands(&[
                    EditCommand::Clear,
                    EditCommand::InsertString(repl.buffer.to_string()),
                    EditCommand::MoveToPosition(repl.cursor_pos),
                ]);
                repl.buffer = "".to_string();
                repl.cursor_pos = 0;
                drop(repl);
            }
            Ok(Signal::CtrlC) => {
                // `Reedline` clears the line content. New prompt is shown
                if shell_integration {
                    run_ansi_sequence(&get_command_finished_marker(stack, engine_state))?;
                }
            }
            Ok(Signal::CtrlD) => {
                // When exiting clear to a new line
                if shell_integration {
                    run_ansi_sequence(&get_command_finished_marker(stack, engine_state))?;
                }
                println!();
                break;
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
                    run_ansi_sequence(&get_command_finished_marker(stack, engine_state))?;
                }
            }
        }
        perf(
            "processing line editor input",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

        perf(
            "finished repl loop",
            loop_start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }

    Ok(())
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
    history_path: &Path,
    line_editor: Reedline,
    history_session_id: Option<HistorySessionId>,
) -> Result<Reedline, ErrReport> {
    let config = engine_state.get_config();
    let history: Box<dyn reedline::History> = match engine_state.config.history_file_format {
        HistoryFileFormat::PlainText => Box::new(
            FileBackedHistory::with_file(
                config.max_history_size as usize,
                history_path.to_path_buf(),
            )
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

pub fn get_command_finished_marker(stack: &Stack, engine_state: &EngineState) -> String {
    let exit_code = stack
        .get_env_var(engine_state, "LAST_EXIT_CODE")
        .and_then(|e| e.as_i64().ok());

    format!("\x1b]133;D;{}\x1b\\", exit_code.unwrap_or(0))
}

fn run_ansi_sequence(seq: &str) -> Result<(), ShellError> {
    io::stdout()
        .write_all(seq.as_bytes())
        .map_err(|e| ShellError::GenericError {
            error: "Error writing ansi sequence".into(),
            msg: e.to_string(),
            span: Some(Span::unknown()),
            help: None,
            inner: vec![],
        })?;
    io::stdout().flush().map_err(|e| ShellError::GenericError {
        error: "Error flushing stdio".into(),
        msg: e.to_string(),
        span: Some(Span::unknown()),
        help: None,
        inner: vec![],
    })
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
    let history_path_o =
        crate::config_files::get_history_path("nushell", engine_state.config.history_file_format);
    assert!(history_path_o.is_some());
    let history_path = history_path_o.as_deref().unwrap();
    let line_editor = reedline::Reedline::create();
    let history_session_id = reedline::Reedline::create_history_session_id();
    let line_editor =
        update_line_editor_history(engine_state, history_path, line_editor, history_session_id);
    assert_eq!(
        i64::from(line_editor.unwrap().get_history_session_id().unwrap()),
        engine_state.history_session_id
    );
}
