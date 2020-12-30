use crate::commands::classified::block::run_block;
use crate::commands::classified::maybe_text_codec::{MaybeTextCodec, StringOrBinary};
use crate::commands::default_context::create_default_context;
use crate::evaluation_context::EvaluationContext;
use crate::path::canonicalize;
use crate::prelude::*;
#[cfg(feature = "rustyline-support")]
use crate::shell::Helper;
use crate::EnvironmentSyncer;
use futures_codec::FramedRead;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::{ClassifiedCommand, Expression, InternalCommand, Literal, NamedArguments};
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};

use log::{debug, trace};
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
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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

pub async fn run_script_standalone(
    script_text: String,
    redirect_stdin: bool,
    context: &EvaluationContext,
    exit_on_error: bool,
) -> Result<(), Box<dyn Error>> {
    let line = process_script(&script_text, context, redirect_stdin, 0, false).await;

    match line {
        LineResult::Success(line) => {
            let error_code = {
                let errors = context.current_errors.clone();
                let errors = errors.lock();

                if errors.len() > 0 {
                    1
                } else {
                    0
                }
            };

            context.maybe_print_errors(Text::from(line));
            if error_code != 0 && exit_on_error {
                std::process::exit(error_code);
            }
        }

        LineResult::Error(line, err) => {
            context.with_host(|_host| {
                print_err(err, &Text::from(line.clone()));
            });

            context.maybe_print_errors(Text::from(line));
            if exit_on_error {
                std::process::exit(1);
            }
        }

        _ => {}
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

fn chomp_newline(s: &str) -> &str {
    if let Some(s) = s.strip_suffix('\n') {
        s
    } else {
        s
    }
}

#[derive(Debug)]
pub enum LineResult {
    Success(String),
    Error(String, ShellError),
    Break,
    CtrlC,
    CtrlD,
    ClearHistory,
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

/// Process the line by parsing the text to turn it into commands, classify those commands so that we understand what is being called in the pipeline, and then run this pipeline
pub async fn process_script(
    script_text: &str,
    ctx: &EvaluationContext,
    redirect_stdin: bool,
    span_offset: usize,
    cli_mode: bool,
) -> LineResult {
    if script_text.trim() == "" {
        LineResult::Success(script_text.to_string())
    } else {
        let line = chomp_newline(script_text);

        let (block, err) = nu_parser::parse(&line, span_offset, &ctx.scope);

        debug!("{:#?}", block);
        //println!("{:#?}", pipeline);

        if let Some(failure) = err {
            return LineResult::Error(line.to_string(), failure.into());
        }

        // There's a special case to check before we process the pipeline:
        // If we're giving a path by itself
        // ...and it's not a command in the path
        // ...and it doesn't have any arguments
        // ...and we're in the CLI
        // ...then change to this directory
        if cli_mode
            && block.block.len() == 1
            && block.block[0].pipelines.len() == 1
            && block.block[0].pipelines[0].list.len() == 1
        {
            if let ClassifiedCommand::Internal(InternalCommand {
                ref name, ref args, ..
            }) = block.block[0].pipelines[0].list[0]
            {
                let internal_name = name;
                let name = args
                    .positional
                    .as_ref()
                    .and_then(|potionals| {
                        potionals.get(0).map(|e| {
                            if let Expression::Literal(Literal::String(ref s)) = e.expr {
                                &s
                            } else {
                                ""
                            }
                        })
                    })
                    .unwrap_or("");

                if internal_name == "run_external"
                    && args
                        .positional
                        .as_ref()
                        .map(|ref v| v.len() == 1)
                        .unwrap_or(true)
                    && args
                        .named
                        .as_ref()
                        .map(NamedArguments::is_empty)
                        .unwrap_or(true)
                    && canonicalize(ctx.shell_manager.path(), name).is_ok()
                    && Path::new(&name).is_dir()
                    && !crate::commands::classified::external::did_find_command(&name)
                {
                    // Here we work differently if we're in Windows because of the expected Windows behavior
                    #[cfg(windows)]
                    {
                        if name.ends_with(':') {
                            // This looks like a drive shortcut. We need to a) switch drives and b) go back to the previous directory we were viewing on that drive
                            // But first, we need to save where we are now
                            let current_path = ctx.shell_manager.path();

                            let split_path: Vec<_> = current_path.split(':').collect();
                            if split_path.len() > 1 {
                                ctx.windows_drives_previous_cwd
                                    .lock()
                                    .insert(split_path[0].to_string(), current_path);
                            }

                            let name = name.to_uppercase();
                            let new_drive: Vec<_> = name.split(':').collect();

                            if let Some(val) =
                                ctx.windows_drives_previous_cwd.lock().get(new_drive[0])
                            {
                                ctx.shell_manager.set_path(val.to_string());
                                return LineResult::Success(line.to_string());
                            } else {
                                ctx.shell_manager
                                    .set_path(format!("{}\\", name.to_string()));
                                return LineResult::Success(line.to_string());
                            }
                        } else {
                            ctx.shell_manager.set_path(name.to_string());
                            return LineResult::Success(line.to_string());
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        ctx.shell_manager.set_path(name.to_string());
                        return LineResult::Success(line.to_string());
                    }
                }
            }
        }

        let input_stream = if redirect_stdin {
            let file = futures::io::AllowStdIo::new(std::io::stdin());
            let stream = FramedRead::new(file, MaybeTextCodec::default()).map(|line| {
                if let Ok(line) = line {
                    let primitive = match line {
                        StringOrBinary::String(s) => Primitive::String(s),
                        StringOrBinary::Binary(b) => Primitive::Binary(b.into_iter().collect()),
                    };

                    Ok(Value {
                        value: UntaggedValue::Primitive(primitive),
                        tag: Tag::unknown(),
                    })
                } else {
                    panic!("Internal error: could not read lines of text from stdin")
                }
            });
            stream.to_input_stream()
        } else {
            InputStream::empty()
        };

        trace!("{:#?}", block);
        let env = ctx.get_env();

        ctx.scope.add_env(env);
        let result = run_block(&block, ctx, input_stream).await;

        match result {
            Ok(input) => {
                // Running a pipeline gives us back a stream that we can then
                // work through. At the top level, we just want to pull on the
                // values to compute them.
                use futures::stream::TryStreamExt;

                let context = RunnableContext {
                    input,
                    shell_manager: ctx.shell_manager.clone(),
                    host: ctx.host.clone(),
                    ctrl_c: ctx.ctrl_c.clone(),
                    current_errors: ctx.current_errors.clone(),
                    scope: ctx.scope.clone(),
                    name: Tag::unknown(),
                };

                if let Ok(mut output_stream) =
                    crate::commands::autoview::command::autoview(context).await
                {
                    loop {
                        match output_stream.try_next().await {
                            Ok(Some(ReturnSuccess::Value(Value {
                                value: UntaggedValue::Error(e),
                                ..
                            }))) => return LineResult::Error(line.to_string(), e),
                            Ok(Some(_item)) => {
                                if ctx.ctrl_c.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => return LineResult::Error(line.to_string(), e),
                        }
                    }
                }

                LineResult::Success(line.to_string())
            }
            Err(err) => LineResult::Error(line.to_string(), err),
        }
    }
}

pub fn print_err(err: ShellError, source: &Text) {
    if let Some(diag) = err.into_diagnostic() {
        let source = source.to_string();
        let mut files = codespan_reporting::files::SimpleFiles::new();
        files.add("shell", source);

        let writer = codespan_reporting::term::termcolor::StandardStream::stderr(
            codespan_reporting::term::termcolor::ColorChoice::Always,
        );
        let config = codespan_reporting::term::Config::default();

        let _ = std::panic::catch_unwind(move || {
            let _ = codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diag);
        });
    }
}

#[cfg(test)]
mod tests {

    #[quickcheck]
    fn quickcheck_parse(data: String) -> bool {
        let (tokens, err) = nu_parser::lex(&data, 0);
        let (lite_block, err2) = nu_parser::group(tokens);
        if err.is_none() && err2.is_none() {
            let context = crate::evaluation_context::EvaluationContext::basic().unwrap();
            let _ = nu_parser::classify_block(&lite_block, &context.scope);
        }
        true
    }
}
