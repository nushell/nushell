use crate::{
    completions::NuCompleter,
    prompt_update,
    reedline_config::{add_menus, create_keybindings, KeybindingsMode},
    util::{eval_source, report_error},
    NuHighlighter, NuValidator, NushellPrompt,
};
use log::{info, trace};
use miette::{IntoDiagnostic, Result};
use nu_color_config::get_color_config;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::lex;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    BlockId, PipelineData, PositionalArg, ShellError, Span, Value,
};
use reedline::{DefaultHinter, Emacs, Vi};
use std::io::{self, Write};
use std::path::PathBuf;
use std::{sync::atomic::Ordering, time::Instant};

const PRE_EXECUTE_MARKER: &str = "\x1b]133;A\x1b\\";
const PRE_PROMPT_MARKER: &str = "\x1b]133;C\x1b\\";
const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

pub fn evaluate_repl(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    history_path: Option<PathBuf>,
    is_perf_true: bool,
) -> Result<()> {
    use reedline::{FileBackedHistory, Reedline, Signal};

    let mut entry_num = 0;

    let mut nu_prompt = NushellPrompt::new();

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
    let mut config = engine_state.get_config();

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

        //Reset the ctrl-c handler
        if let Some(ctrlc) = &mut engine_state.ctrlc {
            ctrlc.store(false, Ordering::SeqCst);
        }

        config = engine_state.get_config();

        if is_perf_true {
            info!("setup colors {}:{}:{}", file!(), line!(), column!());
        }

        let color_hm = get_color_config(config);

        if is_perf_true {
            info!("update reedline {}:{}:{}", file!(), line!(), column!());
        }
        let engine_reference = std::sync::Arc::new(engine_state.clone());
        line_editor = line_editor
            .with_highlighter(Box::new(NuHighlighter {
                engine_state: engine_state.clone(),
                config: config.clone(),
            }))
            .with_animation(config.animate_prompt)
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
            if is_perf_true {
                info!("sync history {}:{}:{}", file!(), line!(), column!());
            }
            line_editor.sync_history().into_diagnostic()?;
        }

        if is_perf_true {
            info!("setup keybindings {}:{}:{}", file!(), line!(), column!());
        }

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

        if is_perf_true {
            info!("prompt_update {}:{}:{}", file!(), line!(), column!());
        }

        // Right before we start our prompt and take input from the user,
        // fire the "pre_prompt" hook
        if let Some(hook) = &config.hooks.pre_prompt {
            if let Err(err) = run_hook(engine_state, stack, vec![], hook) {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &err);
            }
        }

        // Next, check all the environment variables they ask for
        // fire the "env_change" hook
        if let Some(hook) = config.hooks.env_change.clone() {
            match hook {
                Value::Record {
                    cols, vals: blocks, ..
                } => {
                    for (idx, env_var) in cols.iter().enumerate() {
                        let before = engine_state
                            .previous_env_vars
                            .get(env_var)
                            .cloned()
                            .unwrap_or_default();
                        let after = stack.get_env_var(engine_state, env_var).unwrap_or_default();
                        if before != after {
                            if let Err(err) = run_hook(
                                engine_state,
                                stack,
                                vec![before, after.clone()],
                                &blocks[idx],
                            ) {
                                let working_set = StateWorkingSet::new(engine_state);
                                report_error(&working_set, &err);
                            }

                            engine_state
                                .previous_env_vars
                                .insert(env_var.to_string(), after);
                        }
                    }
                }
                x => {
                    let working_set = StateWorkingSet::new(engine_state);
                    report_error(
                        &working_set,
                        &ShellError::TypeMismatch(
                            "record for 'env_change' hook".to_string(),
                            x.span().unwrap_or_else(|_| Span::new(0, 0)),
                        ),
                    )
                }
            }
        }

        config = engine_state.get_config();

        if config.shell_integration {
            run_ansi_sequence(PRE_EXECUTE_MARKER)?;
        }

        let prompt =
            prompt_update::update_prompt(config, engine_state, stack, &mut nu_prompt, is_perf_true);

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
                // Right before we start running the code the user gave us,
                // fire the "pre_execution" hook
                if let Some(hook) = &config.hooks.pre_execution {
                    if let Err(err) = run_hook(engine_state, stack, vec![], hook) {
                        let working_set = StateWorkingSet::new(engine_state);
                        report_error(&working_set, &err);
                    }
                }

                if config.shell_integration {
                    run_ansi_sequence(RESET_APPLICATION_MODE)?;
                    run_ansi_sequence(PRE_PROMPT_MARKER)?;
                    if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
                        let path = cwd.as_string()?;
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
                }

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
                                &ShellError::DirectoryNotFound(tokens.0[0].span, None),
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
                }

                stack.add_env_var(
                    "CMD_DURATION_MS".into(),
                    Value::String {
                        val: format!("{}", start_time.elapsed().as_millis()),
                        span: Span { start: 0, end: 0 },
                    },
                );

                // FIXME: permanent state changes like this hopefully in time can be removed
                // and be replaced by just passing the cwd in where needed
                if let Some(cwd) = stack.get_env_var(engine_state, "PWD") {
                    let path = cwd.as_string()?;
                    let _ = std::env::set_current_dir(path);
                    engine_state.add_env_var("PWD".into(), cwd);
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

pub fn run_hook(
    engine_state: &EngineState,
    stack: &mut Stack,
    arguments: Vec<Value>,
    value: &Value,
) -> Result<(), ShellError> {
    match value {
        Value::List { vals, .. } => {
            for val in vals {
                run_hook(engine_state, stack, arguments.clone(), val)?
            }
            Ok(())
        }
        Value::Block {
            val: block_id,
            span,
            ..
        } => run_hook_block(engine_state, stack, *block_id, arguments, *span),
        x => match x.span() {
            Ok(span) => Err(ShellError::MissingConfigValue(
                "block for hook in config".into(),
                span,
            )),
            _ => Err(ShellError::MissingConfigValue(
                "block for hook in config".into(),
                Span { start: 0, end: 0 },
            )),
        },
    }
}

pub fn run_hook_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block_id: BlockId,
    arguments: Vec<Value>,
    span: Span,
) -> Result<(), ShellError> {
    let block = engine_state.get_block(block_id);
    let input = PipelineData::new(span);

    let mut callee_stack = stack.gather_captures(&block.captures);

    for (idx, PositionalArg { var_id, .. }) in
        block.signature.required_positional.iter().enumerate()
    {
        if let Some(var_id) = var_id {
            callee_stack.add_var(*var_id, arguments[idx].clone())
        }
    }

    match eval_block(engine_state, &mut callee_stack, block, input, false, false) {
        Ok(pipeline_data) => match pipeline_data.into_value(span) {
            Value::Error { error } => Err(error),
            _ => Ok(()),
        },
        Err(err) => Err(err),
    }
}
