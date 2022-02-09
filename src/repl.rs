use crate::is_perf_true;
use crate::reedline_config::{add_completion_menu, add_history_menu};
use crate::{config_files, prompt_update, reedline_config};
use crate::{
    reedline_config::KeybindingsMode,
    utils::{eval_source, gather_parent_env_vars, report_error},
};
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_cli::{NuCompleter, NuHighlighter, NuValidator, NushellPrompt};
use nu_color_config::get_color_config;
use nu_engine::convert_env_values;
use nu_parser::lex;
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Config, ShellError, Span, Value, CONFIG_VARIABLE_ID,
};
use reedline::{DefaultHinter, Emacs, Vi};
use std::{sync::atomic::Ordering, time::Instant};

pub(crate) fn evaluate(engine_state: &mut EngineState) -> Result<()> {
    // use crate::logger::{configure, logger};
    use reedline::{FileBackedHistory, Reedline, Signal};

    let mut entry_num = 0;

    let mut nu_prompt = NushellPrompt::new();
    let mut stack = nu_protocol::engine::Stack::new();

    // First, set up env vars as strings only
    gather_parent_env_vars(engine_state);

    // Set up our initial config to start from
    stack.vars.insert(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    #[cfg(feature = "plugin")]
    config_files::read_plugin_file(engine_state, &mut stack);

    config_files::read_config_file(engine_state, &mut stack);
    let history_path = config_files::create_history_path();

    // Load config struct form config variable
    let config = match stack.get_config() {
        Ok(config) => config,
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &e);
            Config::default()
        }
    };

    // logger(|builder| {
    //     configure(&config.log_level, builder)?;
    //     // trace_filters(self, builder)?;
    //     // debug_filters(self, builder)?;

    //     Ok(())
    // })?;

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, &stack, &config) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
    }

    // seed the cmd_duration_ms env var
    stack.add_env_var(
        "CMD_DURATION_MS".into(),
        Value::String {
            val: "0823".to_string(),
            span: Span { start: 0, end: 0 },
        },
    );

    loop {
        let config = match stack.get_config() {
            Ok(config) => config,
            Err(e) => {
                let working_set = StateWorkingSet::new(engine_state);

                report_error(&working_set, &e);
                Config::default()
            }
        };

        //Reset the ctrl-c handler
        if let Some(ctrlc) = &mut engine_state.ctrlc {
            ctrlc.store(false, Ordering::SeqCst);
        }

        let mut line_editor = Reedline::create()
            .into_diagnostic()?
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_state.clone(),
                config: config.clone(),
            }))
            .with_animation(config.animate_prompt)
            .with_validator(Box::new(NuValidator {
                engine_state: engine_state.clone(),
            }))
            .with_completer(Box::new(NuCompleter::new(engine_state.clone())))
            .with_quick_completions(config.quick_completions)
            .with_ansi_colors(config.use_ansi_coloring);

        line_editor = add_completion_menu(line_editor, &config);
        line_editor = add_history_menu(line_editor, &config);

        //FIXME: if config.use_ansi_coloring is false then we should
        // turn off the hinter but I don't see any way to do that yet.

        let color_hm = get_color_config(&config);

        line_editor = if let Some(history_path) = history_path.clone() {
            let history = std::fs::read_to_string(&history_path);
            if history.is_ok() {
                line_editor
                    .with_hinter(Box::new(
                        DefaultHinter::default().with_style(color_hm["hints"]),
                    ))
                    .with_history(Box::new(
                        FileBackedHistory::with_file(
                            config.max_history_size as usize,
                            history_path.clone(),
                        )
                        .into_diagnostic()?,
                    ))
                    .into_diagnostic()?
            } else {
                line_editor
            }
        } else {
            line_editor
        };

        // Changing the line editor based on the found keybindings
        line_editor = match reedline_config::create_keybindings(&config) {
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

        if is_perf_true() {
            info!("setup line editor {}:{}:{}", file!(), line!(), column!());
        }

        let prompt = prompt_update::update_prompt(&config, engine_state, &stack, &mut nu_prompt);

        if is_perf_true() {
            info!(
                "finished prompt update {}:{}:{}",
                file!(),
                line!(),
                column!()
            );
        }

        entry_num += 1;

        let input = line_editor.read_line(prompt);
        match input {
            Ok(Signal::Success(s)) => {
                let start_time = Instant::now();
                let tokens = lex(s.as_bytes(), 0, &[], &[], false);
                // Check if this is a single call to a directory, if so auto-cd
                let cwd = nu_engine::env::current_dir_str(engine_state, &stack)?;
                let path = nu_path::expand_path_with(&s, &cwd);

                let orig = s.clone();

                if (orig.starts_with('.')
                    || orig.starts_with('~')
                    || orig.starts_with('/')
                    || orig.starts_with('\\'))
                    && path.is_dir()
                    && tokens.0.len() == 1
                {
                    // We have an auto-cd
                    let (path, span) = {
                        if !path.exists() {
                            let working_set = StateWorkingSet::new(engine_state);

                            report_error(
                                &working_set,
                                &ShellError::DirectoryNotFound(tokens.0[0].span),
                            );
                        }

                        let path = nu_path::canonicalize_with(path, &cwd)
                            .expect("internal error: cannot canonicalize known path");
                        (path.to_string_lossy().to_string(), tokens.0[0].span)
                    };

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

                    shells[current_shell] = Value::String { val: path, span };

                    stack.add_env_var("NUSHELL_SHELLS".into(), Value::List { vals: shells, span });
                } else {
                    trace!("eval source: {}", s);

                    eval_source(
                        engine_state,
                        &mut stack,
                        &s,
                        &format!("entry #{}", entry_num),
                    );

                    stack.add_env_var(
                        "CMD_DURATION_MS".into(),
                        Value::String {
                            val: format!("{}", start_time.elapsed().as_millis()),
                            span: Span { start: 0, end: 0 },
                        },
                    );
                }
                // FIXME: permanent state changes like this hopefully in time can be removed
                // and be replaced by just passing the cwd in where needed
                if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
                    let path = cwd.as_string()?;
                    let _ = std::env::set_current_dir(path);
                    engine_state.env_vars.insert("PWD".into(), cwd);
                }
            }
            Ok(Signal::CtrlC) => {
                // `Reedline` clears the line content. New prompt is shown
            }
            Ok(Signal::CtrlD) => {
                // When exiting clear to a new line
                println!();
                break;
            }
            Ok(Signal::CtrlL) => {
                line_editor.clear_screen().into_diagnostic()?;
            }
            Err(err) => {
                let message = err.to_string();
                if !message.contains("duration") {
                    println!("Error: {:?}", err);
                }
            }
        }
    }

    Ok(())
}
