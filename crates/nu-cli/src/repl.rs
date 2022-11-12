use crate::{
    completions::NuCompleter,
    prompt_update,
    reedline_config::{add_menus, create_keybindings, KeybindingsMode},
    util::{eval_source, get_guaranteed_cwd, report_error, report_error_new},
    NuHighlighter, NuValidator, NushellPrompt,
};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use log::{info, trace, warn};
use miette::{IntoDiagnostic, Result};
use nu_color_config::get_color_config;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::{lex, parse, trim_quotes_str};
use nu_protocol::{
    ast::PathMember,
    engine::{EngineState, ReplOperation, Stack, StateWorkingSet},
    format_duration, BlockId, HistoryFileFormat, PipelineData, PositionalArg, ShellError, Span,
    Spanned, Type, Value, VarId,
};
use reedline::{DefaultHinter, EditCommand, Emacs, SqliteBackedHistory, Vi};
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
) -> Result<()> {
    use reedline::{FileBackedHistory, Reedline, Signal};

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

    info!(
        "translate environment vars {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
    }

    // seed env vars
    stack.add_env_var(
        "CMD_DURATION_MS".into(),
        Value::String {
            val: "0823".to_string(),
            span: Span { start: 0, end: 0 },
        },
    );

    stack.add_env_var(
        "LAST_EXIT_CODE".into(),
        Value::Int {
            val: 0,
            span: Span { start: 0, end: 0 },
        },
    );

    info!(
        "load config initially {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    info!("setup reedline {}:{}:{}", file!(), line!(), column!());

    let mut line_editor = Reedline::create();

    // Now that reedline is created, get the history session id and store it in engine_state
    let hist_sesh = match line_editor.get_history_session_id() {
        Some(id) => i64::from(id),
        None => 0,
    };
    engine_state.history_session_id = hist_sesh;

    let config = engine_state.get_config();

    let history_path = crate::config_files::get_history_path(
        nushell_path,
        engine_state.config.history_file_format,
    );
    if let Some(history_path) = history_path.as_deref() {
        info!("setup history {}:{}:{}", file!(), line!(), column!());

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
        line_editor = line_editor.with_history(history);
    };

    let sys = sysinfo::System::new();

    let show_banner = config.show_banner;
    let use_ansi = config.use_ansi_coloring;
    if show_banner {
        let banner = get_banner(engine_state, stack);
        if use_ansi {
            println!("{}", banner);
        } else {
            println!("{}", nu_utils::strip_ansi_string_likely(banner));
        }
    }

    if let Some(s) = prerun_command {
        eval_source(
            engine_state,
            stack,
            s.item.as_bytes(),
            &format!("entry #{}", entry_num),
            PipelineData::new(Span::new(0, 0)),
        );
        engine_state.merge_env(stack, get_guaranteed_cwd(engine_state, stack))?;
    }

    loop {
        info!(
            "load config each loop {}:{}:{}",
            file!(),
            line!(),
            column!()
        );

        let cwd = get_guaranteed_cwd(engine_state, stack);

        // Before doing anything, merge the environment from the previous REPL iteration into the
        // permanent state.
        if let Err(err) = engine_state.merge_env(stack, cwd) {
            report_error_new(engine_state, &err);
        }

        //Reset the ctrl-c handler
        if let Some(ctrlc) = &mut engine_state.ctrlc {
            ctrlc.store(false, Ordering::SeqCst);
        }
        // Reset the SIGQUIT handler
        if let Some(sig_quit) = engine_state.get_sig_quit() {
            sig_quit.store(false, Ordering::SeqCst);
        }

        let config = engine_state.get_config();

        info!("setup colors {}:{}:{}", file!(), line!(), column!());

        let color_hm = get_color_config(config);

        info!("update reedline {}:{}:{}", file!(), line!(), column!());
        let engine_reference = std::sync::Arc::new(engine_state.clone());
        line_editor = line_editor
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_state.clone(),
                config: config.clone(),
            }))
            .with_validator(Box::new(NuValidator {
                engine_state: engine_state.clone(),
            }))
            .with_completer(Box::new(NuCompleter::new(
                engine_reference.clone(),
                stack.clone(),
            )))
            .with_quick_completions(config.quick_completions)
            .with_partial_completions(config.partial_completions)
            .with_ansi_colors(config.use_ansi_coloring);

        line_editor = if config.use_ansi_coloring {
            line_editor.with_hinter(Box::new(
                DefaultHinter::default().with_style(color_hm["hints"]),
            ))
        } else {
            line_editor.disable_hints()
        };

        line_editor = match add_menus(line_editor, engine_reference, stack, config) {
            Ok(line_editor) => line_editor,
            Err(e) => {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
                Reedline::create()
            }
        };

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

        if config.sync_history_on_enter {
            info!("sync history {}:{}:{}", file!(), line!(), column!());

            if let Err(e) = line_editor.sync_history() {
                warn!("Failed to sync history: {}", e);
            }
        }

        info!("setup keybindings {}:{}:{}", file!(), line!(), column!());

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

        info!("prompt_update {}:{}:{}", file!(), line!(), column!());

        // Right before we start our prompt and take input from the user,
        // fire the "pre_prompt" hook
        if let Some(hook) = config.hooks.pre_prompt.clone() {
            if let Err(err) = eval_hook(engine_state, stack, None, vec![], &hook) {
                report_error_new(engine_state, &err);
            }
        }

        // Next, check all the environment variables they ask for
        // fire the "env_change" hook
        let config = engine_state.get_config();
        if let Err(error) =
            eval_env_change_hook(config.hooks.env_change.clone(), engine_state, stack)
        {
            report_error_new(engine_state, &error)
        }

        let config = engine_state.get_config();
        let prompt = prompt_update::update_prompt(config, engine_state, stack, &mut nu_prompt);

        entry_num += 1;

        info!(
            "finished setup, starting repl {}:{}:{}",
            file!(),
            line!(),
            column!()
        );

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

                engine_state
                    .repl_buffer_state
                    .lock()
                    .expect("repl buffer state mutex")
                    .replace(line_editor.current_buffer_contents().to_string());

                // Right before we start running the code the user gave us,
                // fire the "pre_execution" hook
                if let Some(hook) = config.hooks.pre_execution.clone() {
                    if let Err(err) = eval_hook(engine_state, stack, None, vec![], &hook) {
                        report_error_new(engine_state, &err);
                    }
                }

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
                            span: Span { start: 0, end: 0 },
                        },
                    );

                    //FIXME: this only changes the current scope, but instead this environment variable
                    //should probably be a block that loads the information from the state in the overlay
                    stack.add_env_var(
                        "PWD".into(),
                        Value::String {
                            val: path.clone(),
                            span: Span { start: 0, end: 0 },
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
                        &format!("entry #{}", entry_num),
                        PipelineData::new(Span::new(0, 0)),
                    );
                }
                let cmd_duration = start_time.elapsed();

                stack.add_env_var(
                    "CMD_DURATION_MS".into(),
                    Value::String {
                        val: format!("{}", cmd_duration.as_millis()),
                        span: Span { start: 0, end: 0 },
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
                        run_ansi_sequence(&format!("\x1b]2;{}\x07", maybe_abbrev_path))?;
                    }
                    run_ansi_sequence(RESET_APPLICATION_MODE)?;
                }

                let mut ops = engine_state
                    .repl_operation_queue
                    .lock()
                    .expect("repl op queue mutex");
                while let Some(op) = ops.pop_front() {
                    match op {
                        ReplOperation::Append(s) => line_editor.run_edit_commands(&[
                            EditCommand::MoveToEnd,
                            EditCommand::InsertString(s),
                        ]),
                        ReplOperation::Insert(s) => {
                            line_editor.run_edit_commands(&[EditCommand::InsertString(s)])
                        }
                        ReplOperation::Replace(s) => line_editor
                            .run_edit_commands(&[EditCommand::Clear, EditCommand::InsertString(s)]),
                    }
                }
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
                    println!("Error: {:?}", err);
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
    }

    Ok(())
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
Our {}Documentation{} is located at {}http://nushell.sh{}
{}Tweet{} us at {}@nu_shell{}

It's been this long since {}Nushell{}'s first commit:
{}

{}You can disable this banner using the {}config nu{}{} command
to modify the config.nu file and setting show_banner to false.

let-env config = {{
    show_banner: false
    ...
}}{}
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
        "\x1b[32m",   //before Nushell
        "\x1b[0m",    //after Nushell
        age,
        "\x1b[2;37m", //before banner disable dim white
        "\x1b[2;36m", //before config nu dim cyan
        "\x1b[0m",    //after config nu
        "\x1b[2;37m", //after config nu dim white
        "\x1b[0m",    //after banner disable
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
        let (output, _) = parse(&mut working_set, None, source.as_bytes(), false, &[]);

        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let input_as_pipeline_data = match input {
        Some(input) => PipelineData::Value(input, None),
        None => PipelineData::new(Span::test_data()),
    };

    eval_block(
        engine_state,
        stack,
        &block,
        input_as_pipeline_data,
        false,
        true,
    )
    .map(|x| x.into_value(Span::test_data()))
}

pub fn get_command_finished_marker(stack: &Stack, engine_state: &EngineState) -> String {
    let exit_code = stack
        .get_env_var(engine_state, "LAST_EXIT_CODE")
        .and_then(|e| e.as_i64().ok());

    format!("\x1b]133;D;{}\x1b\\", exit_code.unwrap_or(0))
}

pub fn eval_env_change_hook(
    env_change_hook: Option<Value>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    if let Some(hook) = env_change_hook {
        match hook {
            Value::Record {
                cols: env_names,
                vals: hook_values,
                ..
            } => {
                for (env_name, hook_value) in env_names.iter().zip(hook_values.iter()) {
                    let before = engine_state
                        .previous_env_vars
                        .get(env_name)
                        .cloned()
                        .unwrap_or_default();

                    let after = stack
                        .get_env_var(engine_state, env_name)
                        .unwrap_or_default();

                    if before != after {
                        eval_hook(
                            engine_state,
                            stack,
                            None,
                            vec![("$before".into(), before), ("$after".into(), after.clone())],
                            hook_value,
                        )?;

                        engine_state
                            .previous_env_vars
                            .insert(env_name.to_string(), after);
                    }
                }
            }
            x => {
                return Err(ShellError::TypeMismatch(
                    "record for the 'env_change' hook".to_string(),
                    x.span()?,
                ));
            }
        }
    }

    Ok(())
}

pub fn eval_hook(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: Option<PipelineData>,
    arguments: Vec<(String, Value)>,
    value: &Value,
) -> Result<PipelineData, ShellError> {
    let value_span = value.span()?;

    let condition_path = PathMember::String {
        val: "condition".to_string(),
        span: value_span,
    };
    let mut output = PipelineData::new(Span::new(0, 0));

    let code_path = PathMember::String {
        val: "code".to_string(),
        span: value_span,
    };

    match value {
        Value::List { vals, .. } => {
            for val in vals {
                eval_hook(engine_state, stack, None, arguments.clone(), val)?;
            }
        }
        Value::Record { .. } => {
            let do_run_hook =
                if let Ok(condition) = value.clone().follow_cell_path(&[condition_path], false) {
                    match condition {
                        Value::Block {
                            val: block_id,
                            span: block_span,
                            ..
                        }
                        | Value::Closure {
                            val: block_id,
                            span: block_span,
                            ..
                        } => {
                            match run_hook_block(
                                engine_state,
                                stack,
                                block_id,
                                None,
                                arguments.clone(),
                                block_span,
                            ) {
                                Ok(value) => match value {
                                    Value::Bool { val, .. } => val,
                                    other => {
                                        return Err(ShellError::UnsupportedConfigValue(
                                            "boolean output".to_string(),
                                            format!("{}", other.get_type()),
                                            other.span()?,
                                        ));
                                    }
                                },
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        other => {
                            return Err(ShellError::UnsupportedConfigValue(
                                "block".to_string(),
                                format!("{}", other.get_type()),
                                other.span()?,
                            ));
                        }
                    }
                } else {
                    // always run the hook
                    true
                };

            if do_run_hook {
                match value.clone().follow_cell_path(&[code_path], false)? {
                    Value::String {
                        val,
                        span: source_span,
                    } => {
                        let (block, delta, vars) = {
                            let mut working_set = StateWorkingSet::new(engine_state);

                            let mut vars: Vec<(VarId, Value)> = vec![];

                            for (name, val) in arguments {
                                let var_id = working_set.add_variable(
                                    name.as_bytes().to_vec(),
                                    val.span()?,
                                    Type::Any,
                                    false,
                                );

                                vars.push((var_id, val));
                            }

                            let (output, err) =
                                parse(&mut working_set, Some("hook"), val.as_bytes(), false, &[]);
                            if let Some(err) = err {
                                report_error(&working_set, &err);

                                return Err(ShellError::UnsupportedConfigValue(
                                    "valid source code".into(),
                                    "source code with syntax errors".into(),
                                    source_span,
                                ));
                            }

                            (output, working_set.render(), vars)
                        };

                        engine_state.merge_delta(delta)?;
                        let input = PipelineData::new(value_span);

                        let var_ids: Vec<VarId> = vars
                            .into_iter()
                            .map(|(var_id, val)| {
                                stack.add_var(var_id, val);
                                var_id
                            })
                            .collect();

                        match eval_block(engine_state, stack, &block, input, false, false) {
                            Ok(pipeline_data) => {
                                output = pipeline_data;
                            }
                            Err(err) => {
                                report_error_new(engine_state, &err);
                            }
                        }

                        for var_id in var_ids.iter() {
                            stack.vars.remove(var_id);
                        }
                    }
                    Value::Block {
                        val: block_id,
                        span: block_span,
                        ..
                    } => {
                        run_hook_block(
                            engine_state,
                            stack,
                            block_id,
                            input,
                            arguments,
                            block_span,
                        )?;
                    }
                    Value::Closure {
                        val: block_id,
                        span: block_span,
                        ..
                    } => {
                        run_hook_block(
                            engine_state,
                            stack,
                            block_id,
                            input,
                            arguments,
                            block_span,
                        )?;
                    }
                    other => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "block or string".to_string(),
                            format!("{}", other.get_type()),
                            other.span()?,
                        ));
                    }
                }
            }
        }
        Value::Block {
            val: block_id,
            span: block_span,
            ..
        } => {
            output = PipelineData::Value(
                run_hook_block(
                    engine_state,
                    stack,
                    *block_id,
                    input,
                    arguments,
                    *block_span,
                )?,
                None,
            );
        }
        Value::Closure {
            val: block_id,
            span: block_span,
            ..
        } => {
            output = PipelineData::Value(
                run_hook_block(
                    engine_state,
                    stack,
                    *block_id,
                    input,
                    arguments,
                    *block_span,
                )?,
                None,
            );
        }
        other => {
            return Err(ShellError::UnsupportedConfigValue(
                "block, record, or list of records".into(),
                format!("{}", other.get_type()),
                other.span()?,
            ));
        }
    }

    let cwd = get_guaranteed_cwd(engine_state, stack);
    engine_state.merge_env(stack, cwd)?;

    Ok(output)
}

pub fn run_hook_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block_id: BlockId,
    optional_input: Option<PipelineData>,
    arguments: Vec<(String, Value)>,
    span: Span,
) -> Result<Value, ShellError> {
    let block = engine_state.get_block(block_id);

    let input = optional_input.unwrap_or_else(|| PipelineData::new(span));

    let mut callee_stack = stack.gather_captures(&block.captures);

    for (idx, PositionalArg { var_id, .. }) in
        block.signature.required_positional.iter().enumerate()
    {
        if let Some(var_id) = var_id {
            if let Some(arg) = arguments.get(idx) {
                callee_stack.add_var(*var_id, arg.1.clone())
            } else {
                return Err(ShellError::IncompatibleParametersSingle(
                    "This hook block has too many parameters".into(),
                    span,
                ));
            }
        }
    }

    match eval_block(engine_state, &mut callee_stack, block, input, false, false) {
        Ok(pipeline_data) => match pipeline_data.into_value(span) {
            Value::Error { error } => Err(error),
            val => {
                // If all went fine, preserve the environment of the called block
                let caller_env_vars = stack.get_env_var_names(engine_state);

                // remove env vars that are present in the caller but not in the callee
                // (the callee hid them)
                for var in caller_env_vars.iter() {
                    if !callee_stack.has_env_var(engine_state, var) {
                        stack.remove_env_var(engine_state, var);
                    }
                }

                // add new env vars from callee to caller
                for (var, value) in callee_stack.get_stack_env_vars() {
                    stack.add_env_var(var, value);
                }

                Ok(val)
            }
        },
        Err(err) => Err(err),
    }
}

fn run_ansi_sequence(seq: &str) -> Result<(), ShellError> {
    match io::stdout().write_all(seq.as_bytes()) {
        Ok(it) => it,
        Err(err) => {
            return Err(ShellError::GenericError(
                "Error writing ansi sequence".into(),
                err.to_string(),
                Some(Span { start: 0, end: 0 }),
                None,
                Vec::new(),
            ));
        }
    };
    io::stdout().flush().map_err(|e| {
        ShellError::GenericError(
            "Error flushing stdio".into(),
            e.to_string(),
            Some(Span { start: 0, end: 0 }),
            None,
            Vec::new(),
        )
    })
}

lazy_static! {
    // Absolute paths with a drive letter, like 'C:', 'D:\', 'E:\foo'
    static ref DRIVE_PATH_REGEX: Regex =
        Regex::new(r"^[a-zA-Z]:[/\\]?").expect("Internal error: regex creation");
}

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
