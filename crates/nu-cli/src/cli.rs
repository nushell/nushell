use crate::commands::classified::block::run_block;
use crate::commands::default_context::create_default_context;
use crate::evaluation_context::EvaluationContext;
use crate::prelude::*;
use crate::script::{print_err, run_script_standalone};

#[allow(unused_imports)]
pub(crate) use crate::script::{process_script, LineResult};

#[cfg(feature = "rustyline-support")]
use crate::shell::Helper;
use crate::EnvironmentSyncer;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::{UntaggedValue, Value};

#[cfg(feature = "rustyline-support")]
use rustyline::{
    self,
    config::Configurer,
    config::{ColorMode, CompletionType, Config},
    error::ReadlineError,
    At, Cmd, Editor, KeyPress, Movement, Word,
};
use std::error::Error;
use std::iter::Iterator;
use std::path::PathBuf;

pub fn search_paths() -> Vec<std::path::PathBuf> {
    use std::env;

    let mut search_paths = Vec::new();

    // Automatically add path `nu` is in as a search path
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.to_path_buf());
        }
    }

    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
        if let Some(plugin_dirs) = config.get("plugin_dirs") {
            if let Value {
                value: UntaggedValue::Table(pipelines),
                ..
            } = plugin_dirs
            {
                for pipeline in pipelines {
                    if let Ok(plugin_dir) = pipeline.as_string() {
                        search_paths.push(PathBuf::from(plugin_dir));
                    }
                }
            }
        }
    }

    search_paths
}

pub async fn run_script_file(
    file_contents: String,
    redirect_stdin: bool,
) -> Result<(), Box<dyn Error>> {
    let mut syncer = EnvironmentSyncer::new();
    let mut context = create_default_context(false)?;
    let config = syncer.get_config();

    context.configure(&config, |_, ctx| {
        syncer.load_environment();
        syncer.sync_env_vars(ctx);
        syncer.sync_path_vars(ctx);

        if let Err(reason) = syncer.autoenv(ctx) {
            print_err(reason, &Text::from(""));
        }

        let _ = register_plugins(ctx);
        let _ = configure_ctrl_c(ctx);
    });

    let _ = run_startup_commands(&mut context, &config).await;

    run_script_standalone(file_contents, redirect_stdin, &context, true).await?;

    Ok(())
}

#[cfg(feature = "rustyline-support")]
fn convert_rustyline_result_to_string(input: Result<String, ReadlineError>) -> LineResult {
    match input {
        Ok(s) if s == "history -c" || s == "history --clear" => LineResult::ClearHistory,
        Ok(s) => LineResult::Success(s),
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::CtrlD,
        Err(err) => {
            outln!("Error: {:?}", err);
            LineResult::Break
        }
    }
}

/// The entry point for the CLI. Will register all known internal commands, load experimental commands, load plugins, then prepare the prompt and line reader for input.
#[cfg(feature = "rustyline-support")]
pub async fn cli(mut context: EvaluationContext) -> Result<(), Box<dyn Error>> {
    let mut syncer = EnvironmentSyncer::new();
    let configuration = syncer.get_config();

    let mut rl = default_rustyline_editor_configuration();

    context.configure(&configuration, |config, ctx| {
        syncer.load_environment();
        syncer.sync_env_vars(ctx);
        syncer.sync_path_vars(ctx);

        if let Err(reason) = syncer.autoenv(ctx) {
            print_err(reason, &Text::from(""));
        }

        let _ = configure_ctrl_c(ctx);
        let _ = configure_rustyline_editor(&mut rl, config);

        let helper = Some(nu_line_editor_helper(ctx, config));
        rl.set_helper(helper);
    });

    let _ = run_startup_commands(&mut context, &configuration).await;

    // Give ourselves a scope to work in
    context.scope.enter_scope();

    let history_path = crate::commands::history::history_path(&configuration);
    let _ = rl.load_history(&history_path);

    let mut session_text = String::new();
    let mut line_start: usize = 0;

    let skip_welcome_message = configuration
        .var("skip_welcome_message")
        .map(|x| x.is_true())
        .unwrap_or(false);
    if !skip_welcome_message {
        println!(
            "Welcome to Nushell {} (type 'help' for more info)",
            clap::crate_version!()
        );
    }

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    let mut ctrlcbreak = false;

    loop {
        if context.ctrl_c.load(Ordering::SeqCst) {
            context.ctrl_c.store(false, Ordering::SeqCst);
            continue;
        }

        let cwd = context.shell_manager.path();

        let colored_prompt = {
            if let Some(prompt) = configuration.var("prompt") {
                let prompt_line = prompt.as_string()?;

                context.scope.enter_scope();
                let (prompt_block, err) = nu_parser::parse(&prompt_line, 0, &context.scope);

                if err.is_some() {
                    use crate::git::current_branch;
                    context.scope.exit_scope();

                    format!(
                        "\x1b[32m{}{}\x1b[m> ",
                        cwd,
                        match current_branch() {
                            Some(s) => format!("({})", s),
                            None => "".to_string(),
                        }
                    )
                } else {
                    // let env = context.get_env();

                    let run_result = run_block(&prompt_block, &context, InputStream::empty()).await;
                    context.scope.exit_scope();

                    match run_result {
                        Ok(result) => match result.collect_string(Tag::unknown()).await {
                            Ok(string_result) => {
                                let errors = context.get_errors();
                                context.maybe_print_errors(Text::from(prompt_line));
                                context.clear_errors();

                                if !errors.is_empty() {
                                    "> ".to_string()
                                } else {
                                    string_result.item
                                }
                            }
                            Err(e) => {
                                crate::cli::print_err(e, &Text::from(prompt_line));
                                context.clear_errors();

                                "> ".to_string()
                            }
                        },
                        Err(e) => {
                            crate::cli::print_err(e, &Text::from(prompt_line));
                            context.clear_errors();

                            "> ".to_string()
                        }
                    }
                }
            } else {
                use crate::git::current_branch;
                format!(
                    "\x1b[32m{}{}\x1b[m> ",
                    cwd,
                    match current_branch() {
                        Some(s) => format!("({})", s),
                        None => "".to_string(),
                    }
                )
            }
        };

        let prompt = {
            if let Ok(bytes) = strip_ansi_escapes::strip(&colored_prompt) {
                String::from_utf8_lossy(&bytes).to_string()
            } else {
                "> ".to_string()
            }
        };

        rl.helper_mut().expect("No helper").colored_prompt = colored_prompt;
        let mut initial_command = Some(String::new());
        let mut readline = Err(ReadlineError::Eof);
        while let Some(ref cmd) = initial_command {
            readline = rl.readline_with_initial(&prompt, (&cmd, ""));
            initial_command = None;
        }

        if let Ok(line) = &readline {
            line_start = session_text.len();
            session_text.push_str(line);
            session_text.push('\n');
        }

        let line = match convert_rustyline_result_to_string(readline) {
            LineResult::Success(_) => {
                process_script(
                    &session_text[line_start..],
                    &context,
                    false,
                    line_start,
                    true,
                )
                .await
            }
            x => x,
        };

        // Check the config to see if we need to update the path
        // TODO: make sure config is cached so we don't path this load every call
        // FIXME: we probably want to be a bit more graceful if we can't set the environment

        context.configure(&configuration, |config, ctx| {
            if syncer.did_config_change() {
                syncer.reload();
                syncer.sync_env_vars(ctx);
                syncer.sync_path_vars(ctx);
            }

            if let Err(reason) = syncer.autoenv(ctx) {
                print_err(reason, &Text::from(""));
            }

            let _ = configure_rustyline_editor(&mut rl, config);
        });

        match line {
            LineResult::Success(line) => {
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);
                context.maybe_print_errors(Text::from(session_text.clone()));
            }

            LineResult::ClearHistory => {
                rl.clear_history();
                let _ = rl.save_history(&history_path);
            }

            LineResult::Error(line, err) => {
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);

                context.with_host(|_host| {
                    print_err(err, &Text::from(session_text.clone()));
                });

                context.maybe_print_errors(Text::from(session_text.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| s.value.is_true())
                    .unwrap_or(false); // default behavior is to allow CTRL-C spamming similar to other shells

                if !config_ctrlc_exit {
                    continue;
                }

                if ctrlcbreak {
                    let _ = rl.save_history(&history_path);
                    std::process::exit(0);
                } else {
                    context.with_host(|host| host.stdout("CTRL-C pressed (again to quit)"));
                    ctrlcbreak = true;
                    continue;
                }
            }

            LineResult::CtrlD => {
                context.shell_manager.remove_at_current();
                if context.shell_manager.is_empty() {
                    break;
                }
            }

            LineResult::Break => {
                break;
            }
        }
        ctrlcbreak = false;
    }

    // we are ok if we can not save history
    let _ = rl.save_history(&history_path);

    Ok(())
}

pub fn register_plugins(context: &mut EvaluationContext) -> Result<(), ShellError> {
    if let Ok(plugins) = crate::plugin::scan(search_paths()) {
        context.add_commands(
            plugins
                .into_iter()
                .filter(|p| !context.is_command_registered(p.name()))
                .collect(),
        );
    }

    Ok(())
}

fn configure_ctrl_c(_context: &mut EvaluationContext) -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "ctrlc")]
    {
        let cc = _context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })?;

        if _context.ctrl_c.load(Ordering::SeqCst) {
            _context.ctrl_c.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}

async fn run_startup_commands(
    context: &mut EvaluationContext,
    config: &dyn nu_data::config::Conf,
) -> Result<(), ShellError> {
    if let Some(commands) = config.var("startup") {
        match commands {
            Value {
                value: UntaggedValue::Table(pipelines),
                ..
            } => {
                for pipeline in pipelines {
                    if let Ok(pipeline_string) = pipeline.as_string() {
                        let _ = run_script_standalone(pipeline_string, false, context, false).await;
                    }
                }
            }
            _ => {
                return Err(ShellError::untagged_runtime_error(
                    "expected a table of pipeline strings as startup commands",
                ))
            }
        }
    }

    Ok(())
}

#[cfg(feature = "rustyline-support")]
fn default_rustyline_editor_configuration() -> Editor<Helper> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let mut rl: Editor<_> = Editor::with_config(config);

    // add key bindings to move over a whole word with Ctrl+ArrowLeft and Ctrl+ArrowRight
    rl.bind_sequence(
        KeyPress::ControlLeft,
        Cmd::Move(Movement::BackwardWord(1, Word::Vi)),
    );
    rl.bind_sequence(
        KeyPress::ControlRight,
        Cmd::Move(Movement::ForwardWord(1, At::AfterEnd, Word::Vi)),
    );

    // workaround for multiline-paste hang in rustyline (see https://github.com/kkawakam/rustyline/issues/202)
    rl.bind_sequence(KeyPress::BracketedPasteStart, rustyline::Cmd::Noop);

    // Let's set the defaults up front and then override them later if the user indicates
    // defaults taken from here https://github.com/kkawakam/rustyline/blob/2fe886c9576c1ea13ca0e5808053ad491a6fe049/src/config.rs#L150-L167
    rl.set_max_history_size(100);
    rl.set_history_ignore_dups(true);
    rl.set_history_ignore_space(false);
    rl.set_completion_type(DEFAULT_COMPLETION_MODE);
    rl.set_completion_prompt_limit(100);
    rl.set_keyseq_timeout(-1);
    rl.set_edit_mode(rustyline::config::EditMode::Emacs);
    rl.set_auto_add_history(false);
    rl.set_bell_style(rustyline::config::BellStyle::default());
    rl.set_color_mode(rustyline::ColorMode::Enabled);
    rl.set_tab_stop(8);

    if let Err(e) = crate::keybinding::load_keybindings(&mut rl) {
        println!("Error loading keybindings: {:?}", e);
    }

    rl
}

#[cfg(feature = "rustyline-support")]
fn configure_rustyline_editor(
    rl: &mut Editor<Helper>,
    config: &dyn nu_data::config::Conf,
) -> Result<(), ShellError> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    if let Some(line_editor_vars) = config.var("line_editor") {
        for (idx, value) in line_editor_vars.row_entries() {
            match idx.as_ref() {
                "max_history_size" => {
                    if let Ok(max_history_size) = value.as_u64() {
                        rl.set_max_history_size(max_history_size as usize);
                    }
                }
                "history_duplicates" => {
                    // history_duplicates = match value.as_string() {
                    //     Ok(s) if s.to_lowercase() == "alwaysadd" => {
                    //         rustyline::config::HistoryDuplicates::AlwaysAdd
                    //     }
                    //     Ok(s) if s.to_lowercase() == "ignoreconsecutive" => {
                    //         rustyline::config::HistoryDuplicates::IgnoreConsecutive
                    //     }
                    //     _ => rustyline::config::HistoryDuplicates::AlwaysAdd,
                    // };
                    if let Ok(history_duplicates) = value.as_bool() {
                        rl.set_history_ignore_dups(history_duplicates);
                    }
                }
                "history_ignore_space" => {
                    if let Ok(history_ignore_space) = value.as_bool() {
                        rl.set_history_ignore_space(history_ignore_space);
                    }
                }
                "completion_type" => {
                    let completion_type = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "circular" => {
                            rustyline::config::CompletionType::Circular
                        }
                        Ok(s) if s.to_lowercase() == "list" => {
                            rustyline::config::CompletionType::List
                        }
                        #[cfg(all(unix, feature = "with-fuzzy"))]
                        Ok(s) if s.to_lowercase() == "fuzzy" => {
                            rustyline::config::CompletionType::Fuzzy
                        }
                        _ => DEFAULT_COMPLETION_MODE,
                    };
                    rl.set_completion_type(completion_type);
                }
                "completion_prompt_limit" => {
                    if let Ok(completion_prompt_limit) = value.as_u64() {
                        rl.set_completion_prompt_limit(completion_prompt_limit as usize);
                    }
                }
                "keyseq_timeout_ms" => {
                    if let Ok(keyseq_timeout_ms) = value.as_u64() {
                        rl.set_keyseq_timeout(keyseq_timeout_ms as i32);
                    }
                }
                "edit_mode" => {
                    let edit_mode = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "vi" => rustyline::config::EditMode::Vi,
                        Ok(s) if s.to_lowercase() == "emacs" => rustyline::config::EditMode::Emacs,
                        _ => rustyline::config::EditMode::Emacs,
                    };
                    rl.set_edit_mode(edit_mode);
                    // Note: When edit_mode is Emacs, the keyseq_timeout_ms is set to -1
                    // no matter what you may have configured. This is so that key chords
                    // can be applied without having to do them in a given timeout. So,
                    // it essentially turns off the keyseq timeout.
                }
                "auto_add_history" => {
                    if let Ok(auto_add_history) = value.as_bool() {
                        rl.set_auto_add_history(auto_add_history);
                    }
                }
                "bell_style" => {
                    let bell_style = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "audible" => {
                            rustyline::config::BellStyle::Audible
                        }
                        Ok(s) if s.to_lowercase() == "none" => rustyline::config::BellStyle::None,
                        Ok(s) if s.to_lowercase() == "visible" => {
                            rustyline::config::BellStyle::Visible
                        }
                        _ => rustyline::config::BellStyle::default(),
                    };
                    rl.set_bell_style(bell_style);
                }
                "color_mode" => {
                    let color_mode = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "enabled" => rustyline::ColorMode::Enabled,
                        Ok(s) if s.to_lowercase() == "forced" => rustyline::ColorMode::Forced,
                        Ok(s) if s.to_lowercase() == "disabled" => rustyline::ColorMode::Disabled,
                        _ => rustyline::ColorMode::Enabled,
                    };
                    rl.set_color_mode(color_mode);
                }
                "tab_stop" => {
                    if let Ok(tab_stop) = value.as_u64() {
                        rl.set_tab_stop(tab_stop as usize);
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}

#[cfg(feature = "rustyline-support")]
fn nu_line_editor_helper(
    context: &mut EvaluationContext,
    config: &dyn nu_data::config::Conf,
) -> crate::shell::Helper {
    let hinter = rustyline_hinter(config);
    crate::shell::Helper::new(context.clone(), hinter)
}

#[cfg(feature = "rustyline-support")]
fn rustyline_hinter(config: &dyn nu_data::config::Conf) -> Option<rustyline::hint::HistoryHinter> {
    if let Some(line_editor_vars) = config.var("line_editor") {
        for (idx, value) in line_editor_vars.row_entries() {
            if idx == "show_hints" && value.expect_string() == "false" {
                return None;
            }
        }
    }

    Some(rustyline::hint::HistoryHinter {})
}

pub async fn parse_and_eval(line: &str, ctx: &EvaluationContext) -> Result<String, ShellError> {
    // FIXME: do we still need this?
    let line = if let Some(s) = line.strip_suffix('\n') {
        s
    } else {
        line
    };

    // TODO ensure the command whose examples we're testing is actually in the pipeline
    ctx.scope.enter_scope();
    let (classified_block, err) = nu_parser::parse(&line, 0, &ctx.scope);
    if let Some(err) = err {
        ctx.scope.exit_scope();
        return Err(err.into());
    }

    let input_stream = InputStream::empty();
    let env = ctx.get_env();
    ctx.scope.add_env(env);

    let result = run_block(&classified_block, ctx, input_stream).await;
    ctx.scope.exit_scope();

    result?.collect_string(Tag::unknown()).await.map(|x| x.item)
}

#[cfg(test)]
mod tests {

    #[quickcheck]
    fn quickcheck_parse(data: String) -> bool {
        let (tokens, err) = nu_parser::lex(&data, 0);
        let (lite_block, err2) = nu_parser::block(tokens);
        if err.is_none() && err2.is_none() {
            let context = crate::evaluation_context::EvaluationContext::basic().unwrap();
            let _ = nu_parser::classify_block(&lite_block, &context.scope);
        }
        true
    }
}
