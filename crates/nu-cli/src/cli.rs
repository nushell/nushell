use crate::line_editor::configure_ctrl_c;
use nu_command::commands::default_context::create_default_context;
use nu_engine::{
    filesystem::filesystem_shell::FilesystemShellMode, maybe_print_errors, run_block, script,
    EvaluationContext,
};
use std::error::Error;

#[allow(unused_imports)]
pub(crate) use nu_engine::script::{process_script, LineResult};

#[cfg(feature = "rustyline-support")]
use crate::line_editor::{
    configure_rustyline_editor, convert_rustyline_result_to_string,
    default_rustyline_editor_configuration, nu_line_editor_helper,
};

#[allow(unused_imports)]
use nu_data::config;
use nu_source::{Tag, Text};
use nu_stream::InputStream;
use std::ffi::OsString;
#[allow(unused_imports)]
use std::sync::atomic::Ordering;

#[cfg(feature = "rustyline-support")]
use rustyline::{self, error::ReadlineError};

use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::{
    hir::ExternalRedirection, ConfigPath, NuScript, RunScriptOptions, UntaggedValue, Value,
};

use log::trace;
use std::iter::Iterator;
use std::path::PathBuf;

pub struct Options {
    pub config: Option<OsString>,
    pub stdin: bool,
    pub scripts: Vec<NuScript>,
    pub save_history: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Self {
        Self {
            config: None,
            stdin: false,
            scripts: vec![],
            save_history: true,
        }
    }
}

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
        if let Some(Value {
            value: UntaggedValue::Table(pipelines),
            ..
        }) = config.get("plugin_dirs")
        {
            for pipeline in pipelines {
                if let Ok(plugin_dir) = pipeline.as_string() {
                    search_paths.push(PathBuf::from(plugin_dir));
                }
            }
        }
    }

    search_paths
}

pub async fn run_script_file(
    options: Options,
    run_options: RunScriptOptions,
) -> Result<(), Box<dyn Error>> {
    let context = create_default_context(FilesystemShellMode::Script, false)?;

    if let Some(cfg) = options.config {
        load_cfg_as_global_cfg(&context, PathBuf::from(cfg)).await;
    } else {
        load_global_cfg(&context).await;
    }

    let _ = register_plugins(&context);
    let _ = configure_ctrl_c(&context);

    for script in options.scripts {
        script::run_script(script, &run_options, &context).await;
    }

    Ok(())
}

#[cfg(feature = "rustyline-support")]
pub async fn cli(context: EvaluationContext, options: Options) -> Result<(), Box<dyn Error>> {
    let _ = configure_ctrl_c(&context);

    // start time for running startup scripts (this metric includes loading of the cfg, but w/e)
    let startup_commands_start_time = std::time::Instant::now();

    if let Some(cfg) = options.config {
        load_cfg_as_global_cfg(&context, PathBuf::from(cfg)).await;
    } else {
        load_global_cfg(&context).await;
    }
    // Store cmd duration in an env var
    context.scope.add_env_var(
        "CMD_DURATION",
        format!("{:?}", startup_commands_start_time.elapsed()),
    );
    trace!(
        "startup commands took {:?}",
        startup_commands_start_time.elapsed()
    );

    //Configure rustyline
    let mut rl = default_rustyline_editor_configuration();
    let history_path = if let Some(cfg) = &context.configs.lock().global_config {
        let _ = configure_rustyline_editor(&mut rl, cfg);
        let helper = Some(nu_line_editor_helper(&context, cfg));
        rl.set_helper(helper);
        nu_data::config::path::history_path(cfg)
    } else {
        nu_data::config::path::default_history_path()
    };

    // Don't load history if it's not necessary
    if options.save_history {
        let _ = rl.load_history(&history_path);
    }

    //set vars from cfg if present
    let (skip_welcome_message, prompt) = if let Some(cfg) = &context.configs.lock().global_config {
        (
            cfg.var("skip_welcome_message")
                .map(|x| x.is_true())
                .unwrap_or(false),
            cfg.var("prompt"),
        )
    } else {
        (false, None)
    };

    //Check whether dir we start in contains local cfg file and if so load it.
    load_local_cfg_if_present(&context).await;

    // Give ourselves a scope to work in
    context.scope.enter_scope();

    let mut session_text = String::new();
    let mut line_start: usize = 0;

    if !skip_welcome_message {
        println!(
            "Welcome to Nushell {} (type 'help' for more info)",
            clap::crate_version!()
        );
    }

    #[cfg(windows)]
    {
        let _ = nu_ansi_term::enable_ansi_support();
    }

    let mut ctrlcbreak = false;

    let mut run_options = RunScriptOptions::default()
        .cli_mode(true)
        .redirect_stdin(false);

    loop {
        if context.ctrl_c.load(Ordering::SeqCst) {
            context.ctrl_c.store(false, Ordering::SeqCst);
            continue;
        }

        let cwd = context.shell_manager.path();

        let colored_prompt = {
            if let Some(prompt) = &prompt {
                let prompt_line = prompt.as_string()?;

                context.scope.enter_scope();
                let (mut prompt_block, err) = nu_parser::parse(&prompt_line, 0, &context.scope);

                prompt_block.set_redirect(ExternalRedirection::Stdout);

                if err.is_some() {
                    context.scope.exit_scope();

                    format!("\x1b[32m{}{}\x1b[m> ", cwd, current_branch())
                } else {
                    let run_result = run_block(&prompt_block, &context, InputStream::empty()).await;
                    context.scope.exit_scope();

                    match run_result {
                        Ok(result) => match result.collect_string(Tag::unknown()).await {
                            Ok(string_result) => {
                                let errors = context.get_errors();
                                maybe_print_errors(&context, Text::from(prompt_line));
                                context.clear_errors();

                                if !errors.is_empty() {
                                    "> ".to_string()
                                } else {
                                    string_result.item
                                }
                            }
                            Err(e) => {
                                context.host.lock().print_err(e, &Text::from(prompt_line));
                                context.clear_errors();

                                "> ".to_string()
                            }
                        },
                        Err(e) => {
                            context.host.lock().print_err(e, &Text::from(prompt_line));
                            context.clear_errors();

                            "> ".to_string()
                        }
                    }
                }
            } else {
                format!("\x1b[32m{}{}\x1b[m> ", cwd, current_branch())
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

        // start time for command duration
        let cmd_start_time = std::time::Instant::now();

        let line = match convert_rustyline_result_to_string(readline) {
            LineResult::Success(_) => {
                run_options = run_options.span_offset(line_start);
                process_script(&session_text[line_start..], &run_options, &context).await
            }
            x => x,
        };

        // Store cmd duration in an env var
        context
            .scope
            .add_env_var("CMD_DURATION", format!("{:?}", cmd_start_time.elapsed()));

        match line {
            LineResult::Success(line) => {
                if options.save_history {
                    rl.add_history_entry(&line);
                    let _ = rl.save_history(&history_path);
                }
                maybe_print_errors(&context, Text::from(session_text.clone()));
            }

            LineResult::ClearHistory => {
                if options.save_history {
                    rl.clear_history();
                    let _ = rl.save_history(&history_path);
                }
            }

            LineResult::Error(line, err) => {
                if options.save_history {
                    rl.add_history_entry(&line);
                    let _ = rl.save_history(&history_path);
                }

                context
                    .host
                    .lock()
                    .print_err(err, &Text::from(session_text.clone()));

                // I am not so sure, we don't need maybe_print_errors here (as we printed an err
                // above), because maybe_print_errors also clears the errors.
                // TODO Analyze where above err comes from, and whether we need to clear
                // context.errors here
                // Or just be consistent and return errors always in context.errors...
                maybe_print_errors(&context, Text::from(session_text.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = context
                    .configs
                    .lock()
                    .global_config
                    .as_ref()
                    .map(|cfg| cfg.var("ctrlc_exit"))
                    .flatten()
                    .map(|ctrl_c| ctrl_c.is_true())
                    .unwrap_or(false); // default behavior is to allow CTRL-C spamming similar to other shells

                if !config_ctrlc_exit {
                    continue;
                }

                if ctrlcbreak {
                    if options.save_history {
                        let _ = rl.save_history(&history_path);
                    }
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
    if options.save_history {
        let _ = rl.save_history(&history_path);
    }

    Ok(())
}

pub async fn load_local_cfg_if_present(context: &EvaluationContext) {
    trace!("Loading local cfg if present");
    match config::loadable_cfg_exists_in_dir(PathBuf::from(context.shell_manager.path())) {
        Ok(Some(cfg_path)) => {
            if let Err(err) = context.load_config(&ConfigPath::Local(cfg_path)).await {
                context.host.lock().print_err(err, &Text::from(""))
            }
        }
        Err(e) => {
            //Report error while checking for local cfg file
            context.host.lock().print_err(e, &Text::from(""))
        }
        Ok(None) => {
            //No local cfg file present in start dir
        }
    }
}

async fn load_cfg_as_global_cfg(context: &EvaluationContext, path: PathBuf) {
    if let Err(err) = context.load_config(&ConfigPath::Global(path.clone())).await {
        context.host.lock().print_err(err, &Text::from(""));
    } else {
        //TODO current commands assume to find path to global cfg file under config-path
        //TODO use newly introduced nuconfig::file_path instead
        context.scope.add_var(
            "config-path",
            UntaggedValue::filepath(path).into_untagged_value(),
        );
    }
}

pub async fn load_global_cfg(context: &EvaluationContext) {
    match config::default_path() {
        Ok(path) => {
            load_cfg_as_global_cfg(context, path).await;
        }
        Err(e) => {
            context.host.lock().print_err(e, &Text::from(""));
        }
    }
}

pub fn register_plugins(context: &EvaluationContext) -> Result<(), ShellError> {
    if let Ok(plugins) = nu_engine::plugin::build_plugin::scan(search_paths()) {
        context.add_commands(
            plugins
                .into_iter()
                .filter(|p| !context.is_command_registered(p.name()))
                .collect(),
        );
    }

    Ok(())
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

    let result = run_block(&classified_block, ctx, input_stream).await;
    ctx.scope.exit_scope();

    result?.collect_string(Tag::unknown()).await.map(|x| x.item)
}

#[allow(dead_code)]
fn current_branch() -> String {
    #[cfg(feature = "shadow-rs")]
    {
        Some(shadow_rs::branch())
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .map(|x| format!("({})", x))
            .unwrap_or_default()
    }
    #[cfg(not(feature = "shadow-rs"))]
    {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use nu_engine::{basic_evaluation_context, filesystem::filesystem_shell::FilesystemShellMode};

    #[quickcheck]
    fn quickcheck_parse(data: String) -> bool {
        let (tokens, err) = nu_parser::lex(&data, 0);
        let (lite_block, err2) = nu_parser::parse_block(tokens);
        if err.is_none() && err2.is_none() {
            let context = basic_evaluation_context(FilesystemShellMode::Cli).unwrap();
            let _ = nu_parser::classify_block(&lite_block, &context.scope);
        }
        true
    }
}
