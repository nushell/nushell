use crate::reedline_config::add_menus;
use crate::{prompt_update, reedline_config};
use crate::{
    reedline_config::KeybindingsMode,
    util::{eval_source, report_error},
};
use crate::{NuCompleter, NuHighlighter, NuValidator, NushellPrompt};
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_color_config::get_color_config;
use nu_engine::convert_env_values;
use nu_parser::lex;
use nu_protocol::engine::Stack;
use nu_protocol::PipelineData;
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Config, ShellError, Span, Value, CONFIG_VARIABLE_ID,
};
use reedline::{DefaultHinter, Emacs, Vi};
use std::path::PathBuf;
use std::{sync::atomic::Ordering, time::Instant};

pub fn evaluate_repl(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    history_path: Option<PathBuf>,
    is_perf_true: bool,
) -> Result<()> {
    use reedline::{FileBackedHistory, Reedline, Signal};

    let mut entry_num = 0;

    let mut nu_prompt = NushellPrompt::new();

    // First, set up env vars as strings only
    // gather_parent_env_vars(engine_state);

    // Set up our initial config to start from
    // stack.vars.insert(
    //     CONFIG_VARIABLE_ID,
    //     Value::Record {
    //         cols: vec![],
    //         vals: vec![],
    //         span: Span::new(0, 0),
    //     },
    // );

    if is_perf_true {
        info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
    }

    // #[cfg(feature = "plugin")]
    // config_files::read_plugin_file(engine_state, &mut stack, is_perf_true);
    //
    // if is_perf_true {
    //     info!("read_config_file {}:{}:{}", file!(), line!(), column!());
    // }
    //
    // config_files::read_config_file(engine_state, &mut stack, config_file, is_perf_true);
    // let history_path = config_files::create_history_path();

    // logger(|builder| {
    //     configure(&config.log_level, builder)?;
    //     // trace_filters(self, builder)?;
    //     // debug_filters(self, builder)?;

    //     Ok(())
    // })?;

    if is_perf_true {
        info!(
            "translate environment vars {}:{}:{}",
            file!(),
            line!(),
            column!()
        );
    }

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
    }

    // Make a note of the exceptions we see for externals that look like math expressions
    let exceptions = crate::util::external_exceptions(engine_state, stack);
    engine_state.external_exceptions = exceptions;

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

    if is_perf_true {
        info!(
            "load config initially {}:{}:{}",
            file!(),
            line!(),
            column!()
        );
    }

    // Get the config once for the history `max_history_size`
    // Updating that will not be possible in one session
    let mut config = match stack.get_config() {
        Ok(config) => config,
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &e);
            Config::default()
        }
    };

    if is_perf_true {
        info!("setup reedline {}:{}:{}", file!(), line!(), column!());
    }
    let mut line_editor = Reedline::create();
    if let Some(history_path) = history_path.as_deref() {
        if is_perf_true {
            info!("setup history {}:{}:{}", file!(), line!(), column!());
        }
        let history = Box::new(
            FileBackedHistory::with_file(
                config.max_history_size as usize,
                history_path.to_path_buf(),
            )
            .into_diagnostic()?,
        );
        line_editor = line_editor.with_history(history);
    };

    loop {
        if is_perf_true {
            info!(
                "load config each loop {}:{}:{}",
                file!(),
                line!(),
                column!()
            );
        }

        config = match stack.get_config() {
            Ok(config) => config,
            Err(e) => {
                let working_set = StateWorkingSet::new(engine_state);

                report_error(&working_set, &e);
                Config::default()
            }
        };
        let color_hm = get_color_config(&config);

        //Reset the ctrl-c handler
        if let Some(ctrlc) = &mut engine_state.ctrlc {
            ctrlc.store(false, Ordering::SeqCst);
        }

        line_editor = line_editor
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_state.clone(),
                config: config.clone(),
            }))
            .with_animation(config.animate_prompt)
            .with_validator(Box::new(NuValidator {
                engine_state: engine_state.clone(),
            }))
            .with_hinter(Box::new(
                DefaultHinter::default().with_style(color_hm["hints"]),
            ))
            .with_completer(Box::new(NuCompleter::new(
                engine_state.clone(),
                stack.clone(),
                stack.vars.get(&CONFIG_VARIABLE_ID).cloned(),
            )))
            .with_quick_completions(config.quick_completions)
            .with_partial_completions(config.partial_completions)
            .with_ansi_colors(config.use_ansi_coloring);

        if is_perf_true {
            info!("update reedline {}:{}:{}", file!(), line!(), column!());
        }

        line_editor = match add_menus(line_editor, engine_state, stack, &config) {
            Ok(line_editor) => line_editor,
            Err(e) => {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
                Reedline::create()
            }
        };

        if is_perf_true {
            info!("setup colors {}:{}:{}", file!(), line!(), column!());
        }
        //FIXME: if config.use_ansi_coloring is false then we should
        // turn off the hinter but I don't see any way to do that yet.

        if config.sync_history_on_enter {
            if is_perf_true {
                info!("sync history {}:{}:{}", file!(), line!(), column!());
            }
            line_editor.sync_history().into_diagnostic()?;
        }

        if is_perf_true {
            info!("setup keybindings {}:{}:{}", file!(), line!(), column!());
        }

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

        if is_perf_true {
            info!("prompt_update {}:{}:{}", file!(), line!(), column!());
        }

        let prompt = prompt_update::update_prompt(
            &config,
            engine_state,
            stack,
            &mut nu_prompt,
            is_perf_true,
        );

        entry_num += 1;

        if is_perf_true {
            info!(
                "finished setup, starting repl {}:{}:{}",
                file!(),
                line!(),
                column!()
            );
        }

        let input = line_editor.read_line(prompt);
        match input {
            Ok(Signal::Success(s)) => {
                let start_time = Instant::now();
                let tokens = lex(s.as_bytes(), 0, &[], &[], false);
                // Check if this is a single call to a directory, if so auto-cd
                let cwd = nu_engine::env::current_dir_str(engine_state, stack)?;
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
                        stack,
                        s.as_bytes(),
                        &format!("entry #{}", entry_num),
                        PipelineData::new(Span::new(0, 0)),
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

                // Make a note of the exceptions we see for externals that look like math expressions
                let exceptions = crate::util::external_exceptions(engine_state, stack);
                engine_state.external_exceptions = exceptions;
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
