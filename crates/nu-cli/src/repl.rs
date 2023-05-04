use crate::{
    completions::NuCompleter,
    prompt_update,
    reedline_config::{add_menus, create_keybindings, KeybindingsMode},
    util::eval_source,
    NuHighlighter, NuValidator, NushellPrompt,
};
use crossterm::cursor::SetCursorStyle;
use log::{trace, warn};
use miette::{IntoDiagnostic, Result};
use nu_color_config::StyleComputer;
use nu_command::hook::eval_hook;
use nu_command::util::get_guaranteed_cwd;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::{lex, parse, trim_quotes_str};
use nu_protocol::{
    config::NuCursorShape,
    engine::{EngineState, Stack, StateWorkingSet},
    format_duration, report_error, report_error_new, HistoryFileFormat, PipelineData, ShellError,
    Span, Spanned, Value,
};
use nu_utils::utils::perf;
use reedline::{CursorConfig, DefaultHinter, EditCommand, Emacs, SqliteBackedHistory, Vi};
use std::{
    io::{self, Write},
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
    entire_start_time: Instant,
) -> Result<()> {
    use nu_command::hook;
    use reedline::{FileBackedHistory, Reedline, Signal};
    let use_color = engine_state.get_config().use_ansi_coloring;

    // Guard against invocation without a connected terminal.
    // reedline / crossterm event polling will fail without a connected tty
    if !atty::is(atty::Stream::Stdin) {
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

    // Now that reedline is created, get the history session id and store it in engine_state
    let hist_sesh = line_editor
        .get_history_session_id()
        .map(i64::from)
        .unwrap_or(0);
    engine_state.history_session_id = hist_sesh;
    perf(
        "setup reedline",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let config = engine_state.get_config();
    if config.bracketed_paste {
        // try to enable bracketed paste
        // It doesn't work on windows system: https://github.com/crossterm-rs/crossterm/issues/737
        #[cfg(not(target_os = "windows"))]
        let _ = line_editor.enable_bracketed_paste();
    }

    // Setup history_isolation aka "history per session"
    let history_isolation = config.history_isolation;
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
        let history: Box<dyn reedline::History> = match engine_state.config.history_file_format {
            HistoryFileFormat::PlainText => Box::new(
                FileBackedHistory::with_file(
                    config.max_history_size as usize,
                    history_path.to_path_buf(),
                )
                .into_diagnostic()?,
            ),
            HistoryFileFormat::Sqlite => Box::new(
                SqliteBackedHistory::with_file(history_path.to_path_buf()).into_diagnostic()?,
            ),
        };
        line_editor = line_editor
            .with_history_session_id(history_session_id)
            .with_history(history);
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

    let show_banner = config.show_banner;
    let use_ansi = config.use_ansi_coloring;
    if show_banner {
        let banner = get_banner(engine_state, stack);
        if use_ansi {
            println!("{banner}");
        } else {
            println!("{}", nu_utils::strip_ansi_string_likely(banner));
        }
    }
    perf(
        "get sysinfo/show banner",
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
            vi_insert: Some(map_nucursorshape_to_cursorshape(
                config.cursor_shape_vi_insert,
            )),
            vi_normal: Some(map_nucursorshape_to_cursorshape(
                config.cursor_shape_vi_normal,
            )),
            emacs: Some(map_nucursorshape_to_cursorshape(config.cursor_shape_emacs)),
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
            .with_cursor_config(cursor_config);
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
                DefaultHinter::default().with_style(style)
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
        let buffer_editor = if !config.buffer_editor.is_empty() {
            Some(config.buffer_editor.clone())
        } else {
            stack
                .get_env_var(engine_state, "EDITOR")
                .map(|v| v.as_string().unwrap_or_default())
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    stack
                        .get_env_var(engine_state, "VISUAL")
                        .map(|v| v.as_string().unwrap_or_default())
                        .filter(|v| !v.is_empty())
                })
        };

        line_editor = if let Some(buffer_editor) = buffer_editor {
            line_editor.with_buffer_editor(buffer_editor, "nu".into())
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
            if let Err(err) = eval_hook(engine_state, stack, None, vec![], &hook) {
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

        if entry_num == 1 {
            engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);
            if show_banner {
                println!(
                    "Startup Time: {}",
                    format_duration(engine_state.get_startup_time())
                );
            }
        }

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
                    let mut repl_buffer = engine_state
                        .repl_buffer_state
                        .lock()
                        .expect("repl buffer state mutex");
                    *repl_buffer = s.to_string();
                    drop(repl_buffer);

                    if let Err(err) = eval_hook(engine_state, stack, None, vec![], &hook) {
                        report_error_new(engine_state, &err);
                    }
                }

                let mut repl_cursor = engine_state
                    .repl_cursor_pos
                    .lock()
                    .expect("repl cursor pos mutex");
                *repl_cursor = line_editor.current_insertion_point();
                drop(repl_cursor);
                let mut repl_buffer = engine_state
                    .repl_buffer_state
                    .lock()
                    .expect("repl buffer state mutex");
                *repl_buffer = line_editor.current_buffer_contents().to_string();
                drop(repl_buffer);

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
                                &ShellError::DirectoryNotFound(tokens.0[0].span, None),
                            );
                        }
                        let path = nu_path::canonicalize_with(path, &cwd)
                            .expect("internal error: cannot canonicalize known path");
                        (path.to_string_lossy().to_string(), tokens.0[0].span)
                    };

                    stack.add_env_var(
                        "OLDPWD".into(),
                        Value::String {
                            val: cwd.clone(),
                            span: Span::unknown(),
                        },
                    );

                    //FIXME: this only changes the current scope, but instead this environment variable
                    //should probably be a block that loads the information from the state in the overlay
                    stack.add_env_var(
                        "PWD".into(),
                        Value::String {
                            val: path.clone(),
                            span: Span::unknown(),
                        },
                    );
                    let cwd = Value::String { val: cwd, span };

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
                        v.as_integer().unwrap_or_default() as usize
                    } else {
                        0
                    };

                    let last_shell = stack.get_env_var(engine_state, "NUSHELL_LAST_SHELL");
                    let last_shell = if let Some(v) = last_shell {
                        v.as_integer().unwrap_or_default() as usize
                    } else {
                        0
                    };

                    shells[current_shell] = Value::String { val: path, span };

                    stack.add_env_var("NUSHELL_SHELLS".into(), Value::List { vals: shells, span });
                    stack.add_env_var(
                        "NUSHELL_LAST_SHELL".into(),
                        Value::Int {
                            val: last_shell as i64,
                            span,
                        },
                    );
                } else if !s.trim().is_empty() {
                    trace!("eval source: {}", s);

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
                    Value::String {
                        val: format!("{}", cmd_duration.as_millis()),
                        span: Span::unknown(),
                    },
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

                        // Communicate the path as OSC 7 (often used for spawning new tabs in the same dir)
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

                let mut repl_buffer = engine_state
                    .repl_buffer_state
                    .lock()
                    .expect("repl buffer state mutex");
                let mut repl_cursor_pos = engine_state
                    .repl_cursor_pos
                    .lock()
                    .expect("repl cursor pos mutex");
                line_editor.run_edit_commands(&[
                    EditCommand::Clear,
                    EditCommand::InsertString(repl_buffer.to_string()),
                    EditCommand::MoveToPosition(*repl_cursor_pos),
                ]);
                *repl_buffer = "".to_string();
                drop(repl_buffer);
                *repl_cursor_pos = 0;
                drop(repl_cursor_pos);
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

fn map_nucursorshape_to_cursorshape(shape: NuCursorShape) -> SetCursorStyle {
    match shape {
        NuCursorShape::Block => SetCursorStyle::SteadyBlock,
        NuCursorShape::UnderScore => SetCursorStyle::SteadyUnderScore,
        NuCursorShape::Line => SetCursorStyle::SteadyBar,
        NuCursorShape::BlinkBlock => SetCursorStyle::BlinkingBlock,
        NuCursorShape::BlinkUnderScore => SetCursorStyle::BlinkingUnderScore,
        NuCursorShape::BlinkLine => SetCursorStyle::BlinkingBar,
    }
}

fn get_banner(engine_state: &mut EngineState, stack: &mut Stack) -> String {
    let age = match eval_string_with_input(
        engine_state,
        stack,
        None,
        "(date now) - ('2019-05-10 09:59:12-0700' | into datetime)",
    ) {
        Ok(Value::Duration { val, .. }) => format_duration(val),
        _ => "".to_string(),
    };

    let banner = format!(
        r#"{}     __  ,
{} .--()°'.' {}Welcome to {}Nushell{},
{}'|, . ,'   {}based on the {}nu{} language,
{} !_-(_\    {}where all data is structured!

Please join our {}Discord{} community at {}https://discord.gg/NtAbbGn{}
Our {}GitHub{} repository is at {}https://github.com/nushell/nushell{}
Our {}Documentation{} is located at {}https://nushell.sh{}
{}Tweet{} us at {}@nu_shell{}
Learn how to remove this at: {}https://nushell.sh/book/configuration.html#remove-welcome-message{}

It's been this long since {}Nushell{}'s first commit:
{}{}
"#,
        "\x1b[32m",   //start line 1 green
        "\x1b[32m",   //start line 2
        "\x1b[0m",    //before welcome
        "\x1b[32m",   //before nushell
        "\x1b[0m",    //after nushell
        "\x1b[32m",   //start line 3
        "\x1b[0m",    //before based
        "\x1b[32m",   //before nu
        "\x1b[0m",    //after nu
        "\x1b[32m",   //start line 4
        "\x1b[0m",    //before where
        "\x1b[35m",   //before Discord purple
        "\x1b[0m",    //after Discord
        "\x1b[35m",   //before Discord URL
        "\x1b[0m",    //after Discord URL
        "\x1b[1;32m", //before GitHub green_bold
        "\x1b[0m",    //after GitHub
        "\x1b[1;32m", //before GitHub URL
        "\x1b[0m",    //after GitHub URL
        "\x1b[32m",   //before Documentation
        "\x1b[0m",    //after Documentation
        "\x1b[32m",   //before Documentation URL
        "\x1b[0m",    //after Documentation URL
        "\x1b[36m",   //before Tweet blue
        "\x1b[0m",    //after Tweet
        "\x1b[1;36m", //before @nu_shell cyan_bold
        "\x1b[0m",    //after @nu_shell
        "\x1b[32m",   //before Welcome Message
        "\x1b[0m",    //after Welcome Message
        "\x1b[32m",   //before Nushell
        "\x1b[0m",    //after Nushell
        age,
        "\x1b[0m", //after banner disable
    );

    banner
}

// Taken from Nana's simple_eval
/// Evaluate a block of Nu code, optionally with input.
/// For example, source="$in * 2" will multiply the value in input by 2.
pub fn eval_string_with_input(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: Option<Value>,
    source: &str,
) -> Result<Value, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(&mut working_set, None, source.as_bytes(), false);

        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let input_as_pipeline_data = match input {
        Some(input) => PipelineData::Value(input, None),
        None => PipelineData::empty(),
    };

    eval_block(
        engine_state,
        stack,
        &block,
        input_as_pipeline_data,
        false,
        true,
    )
    .map(|x| x.into_value(Span::unknown()))
}

pub fn get_command_finished_marker(stack: &Stack, engine_state: &EngineState) -> String {
    let exit_code = stack
        .get_env_var(engine_state, "LAST_EXIT_CODE")
        .and_then(|e| e.as_i64().ok());

    format!("\x1b]133;D;{}\x1b\\", exit_code.unwrap_or(0))
}

fn run_ansi_sequence(seq: &str) -> Result<(), ShellError> {
    io::stdout().write_all(seq.as_bytes()).map_err(|e| {
        ShellError::GenericError(
            "Error writing ansi sequence".into(),
            e.to_string(),
            Some(Span::unknown()),
            None,
            Vec::new(),
        )
    })?;
    io::stdout().flush().map_err(|e| {
        ShellError::GenericError(
            "Error flushing stdio".into(),
            e.to_string(),
            Some(Span::unknown()),
            None,
            Vec::new(),
        )
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
}

#[test]
fn looks_like_path_windows_drive_path_works() {
    let on_windows = cfg!(windows);
    assert_eq!(looks_like_path("C:"), on_windows);
    assert_eq!(looks_like_path("D:\\"), on_windows);
    assert_eq!(looks_like_path("E:/"), on_windows);
    assert_eq!(looks_like_path("F:\\some_dir"), on_windows);
    assert_eq!(looks_like_path("G:/some_dir"), on_windows);
}
