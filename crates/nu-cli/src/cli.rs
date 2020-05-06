use crate::commands::classified::block::run_block;
use crate::commands::classified::external::{MaybeTextCodec, StringOrBinary};
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::whole_stream_command;
use crate::context::Context;
#[cfg(not(feature = "starship-prompt"))]
use crate::git::current_branch;
use crate::path::canonicalize;
use crate::prelude::*;
use futures_codec::FramedRead;

use nu_errors::ShellError;
use nu_protocol::hir::{ClassifiedCommand, Expression, InternalCommand, Literal, NamedArguments};
use nu_protocol::{Primitive, ReturnSuccess, Scope, Signature, UntaggedValue, Value};

use log::{debug, trace};
use rustyline::error::ReadlineError;
use rustyline::{
    self, config::Configurer, config::EditMode, At, Cmd, ColorMode, CompletionType, Config, Editor,
    KeyPress, Movement, Word,
};
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use rayon::prelude::*;

fn load_plugin(path: &std::path::Path, context: &mut Context) -> Result<(), ShellError> {
    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    let stdout = child.stdout.as_mut().expect("Failed to open stdout");

    let mut reader = BufReader::new(stdout);

    let request = JsonRpc::new("config", Vec::<Value>::new());
    let request_raw = serde_json::to_string(&request)?;
    stdin.write_all(format!("{}\n", request_raw).as_bytes())?;
    let path = dunce::canonicalize(path)?;

    let mut input = String::new();
    let result = match reader.read_line(&mut input) {
        Ok(count) => {
            trace!("processing response ({} bytes)", count);
            trace!("response: {}", input);

            let response = serde_json::from_str::<JsonRpc<Result<Signature, ShellError>>>(&input);
            match response {
                Ok(jrpc) => match jrpc.params {
                    Ok(params) => {
                        let fname = path.to_string_lossy();

                        trace!("processing {:?}", params);

                        let name = params.name.clone();
                        let fname = fname.to_string();

                        if context.get_command(&name).is_some() {
                            trace!("plugin {:?} already loaded.", &name);
                        } else if params.is_filter {
                            context.add_commands(vec![whole_stream_command(PluginCommand::new(
                                name, fname, params,
                            ))]);
                        } else {
                            context.add_commands(vec![whole_stream_command(PluginSink::new(
                                name, fname, params,
                            ))]);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                Err(e) => {
                    trace!("incompatible plugin {:?}", input);
                    Err(ShellError::untagged_runtime_error(format!(
                        "Error: {:?}",
                        e
                    )))
                }
            }
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!(
            "Error: {:?}",
            e
        ))),
    };

    let _ = child.wait();

    result
}

fn search_paths() -> Vec<std::path::PathBuf> {
    use std::env;

    let mut search_paths = Vec::new();

    // Automatically add path `nu` is in as a search path
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.to_path_buf());
        }
    }

    #[cfg(not(debug_assertions))]
    {
        match env::var_os("PATH") {
            Some(paths) => {
                search_paths.extend(env::split_paths(&paths).collect::<Vec<_>>());
            }
            None => println!("PATH is not defined in the environment."),
        }
    }

    search_paths
}

pub fn load_plugins(context: &mut Context) -> Result<(), ShellError> {
    let opts = glob::MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    for path in search_paths() {
        let mut pattern = path.to_path_buf();

        pattern.push(std::path::Path::new("nu_plugin_[a-z0-9][a-z0-9]*"));

        let plugs: Vec<_> = glob::glob_with(&pattern.to_string_lossy(), opts)?
            .filter_map(|x| x.ok())
            .collect();

        let _failures: Vec<_> = plugs
            .par_iter()
            .map(|path| {
                let bin_name = {
                    if let Some(name) = path.file_name() {
                        match name.to_str() {
                            Some(raw) => raw,
                            None => "",
                        }
                    } else {
                        ""
                    }
                };

                let is_valid_name = {
                    #[cfg(windows)]
                    {
                        bin_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                    }

                    #[cfg(not(windows))]
                    {
                        bin_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    }
                };

                let is_executable = {
                    #[cfg(windows)]
                    {
                        bin_name.ends_with(".exe") || bin_name.ends_with(".bat")
                    }

                    #[cfg(not(windows))]
                    {
                        true
                    }
                };

                if is_valid_name && is_executable {
                    trace!("Trying {:?}", path.display());

                    // we are ok if this plugin load fails
                    let _ = load_plugin(&path, &mut context.clone());
                }
            })
            .collect();
    }

    Ok(())
}

pub struct History;

impl History {
    pub fn path() -> PathBuf {
        const FNAME: &str = "history.txt";
        config::user_data()
            .map(|mut p| {
                p.push(FNAME);
                p
            })
            .unwrap_or_else(|_| PathBuf::from(FNAME))
    }
}

#[allow(dead_code)]
fn create_default_starship_config() -> Option<toml::Value> {
    let mut map = toml::value::Table::new();
    map.insert("add_newline".into(), toml::Value::Boolean(false));

    let mut git_branch = toml::value::Table::new();
    git_branch.insert("symbol".into(), toml::Value::String("📙 ".into()));
    map.insert("git_branch".into(), toml::Value::Table(git_branch));

    let mut git_status = toml::value::Table::new();
    git_status.insert("disabled".into(), toml::Value::Boolean(true));
    map.insert("git_status".into(), toml::Value::Table(git_status));

    Some(toml::Value::Table(map))
}

pub fn create_default_context(
    syncer: &mut crate::EnvironmentSyncer,
    interactive: bool,
) -> Result<Context, Box<dyn Error>> {
    syncer.load_environment();

    let mut context = Context::basic()?;
    syncer.sync_env_vars(&mut context);
    syncer.sync_path_vars(&mut context);

    {
        use crate::commands::*;

        context.add_commands(vec![
            // System/file operations
            whole_stream_command(Pwd),
            whole_stream_command(Ls),
            whole_stream_command(Du),
            whole_stream_command(Cd),
            whole_stream_command(Remove),
            whole_stream_command(Open),
            whole_stream_command(Config),
            whole_stream_command(Help),
            whole_stream_command(History),
            whole_stream_command(Save),
            whole_stream_command(Touch),
            whole_stream_command(Cpy),
            whole_stream_command(Date),
            whole_stream_command(Calc),
            whole_stream_command(Mkdir),
            whole_stream_command(Move),
            whole_stream_command(Kill),
            whole_stream_command(Version),
            whole_stream_command(Clear),
            whole_stream_command(What),
            whole_stream_command(Which),
            whole_stream_command(Debug),
            whole_stream_command(Alias),
            whole_stream_command(WithEnv),
            // Statistics
            whole_stream_command(Size),
            whole_stream_command(Count),
            // Metadata
            whole_stream_command(Tags),
            // Shells
            whole_stream_command(Next),
            whole_stream_command(Previous),
            whole_stream_command(Shells),
            whole_stream_command(Enter),
            whole_stream_command(Exit),
            // Viewers
            whole_stream_command(Autoview),
            whole_stream_command(Table),
            // Text manipulation
            whole_stream_command(SplitColumn),
            whole_stream_command(SplitRow),
            whole_stream_command(Lines),
            whole_stream_command(Trim),
            whole_stream_command(Echo),
            whole_stream_command(Parse),
            // Column manipulation
            whole_stream_command(Reject),
            whole_stream_command(Pick),
            whole_stream_command(Get),
            whole_stream_command(Edit),
            whole_stream_command(Insert),
            whole_stream_command(SplitBy),
            // Row manipulation
            whole_stream_command(Reverse),
            whole_stream_command(Append),
            whole_stream_command(Prepend),
            whole_stream_command(SortBy),
            whole_stream_command(GroupBy),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(Nth),
            whole_stream_command(Drop),
            whole_stream_command(Format),
            whole_stream_command(Where),
            whole_stream_command(Compact),
            whole_stream_command(Default),
            whole_stream_command(Skip),
            whole_stream_command(SkipUntil),
            whole_stream_command(SkipWhile),
            whole_stream_command(Keep),
            whole_stream_command(KeepUntil),
            whole_stream_command(KeepWhile),
            whole_stream_command(Range),
            whole_stream_command(Rename),
            whole_stream_command(Uniq),
            whole_stream_command(Each),
            whole_stream_command(IsEmpty),
            // Table manipulation
            whole_stream_command(Merge),
            whole_stream_command(Shuffle),
            whole_stream_command(Wrap),
            whole_stream_command(Pivot),
            whole_stream_command(Headers),
            // Data processing
            whole_stream_command(Histogram),
            whole_stream_command(Sum),
            // File format output
            whole_stream_command(To),
            whole_stream_command(ToBSON),
            whole_stream_command(ToCSV),
            whole_stream_command(ToHTML),
            whole_stream_command(ToJSON),
            whole_stream_command(ToSQLite),
            whole_stream_command(ToDB),
            whole_stream_command(ToMarkdown),
            whole_stream_command(ToTOML),
            whole_stream_command(ToTSV),
            whole_stream_command(ToURL),
            whole_stream_command(ToYAML),
            // File format input
            whole_stream_command(From),
            whole_stream_command(FromCSV),
            whole_stream_command(FromEML),
            whole_stream_command(FromTSV),
            whole_stream_command(FromSSV),
            whole_stream_command(FromINI),
            whole_stream_command(FromBSON),
            whole_stream_command(FromJSON),
            whole_stream_command(FromODS),
            whole_stream_command(FromDB),
            whole_stream_command(FromSQLite),
            whole_stream_command(FromTOML),
            whole_stream_command(FromURL),
            whole_stream_command(FromXLSX),
            whole_stream_command(FromXML),
            whole_stream_command(FromYAML),
            whole_stream_command(FromYML),
            whole_stream_command(FromIcs),
            whole_stream_command(FromVcf),
            // "Private" commands (not intended to be accessed directly)
            whole_stream_command(RunExternalCommand { interactive }),
        ]);

        cfg_if::cfg_if! {
            if #[cfg(data_processing_primitives)] {
                context.add_commands(vec![
                whole_stream_command(ReduceBy),
                whole_stream_command(EvaluateBy),
                whole_stream_command(TSortBy),
                whole_stream_command(MapMaxBy),
                ]);
            }
        }

        #[cfg(feature = "clipboard")]
        {
            context.add_commands(vec![whole_stream_command(
                crate::commands::clip::clipboard::Clip,
            )]);
        }
    }

    Ok(context)
}

pub async fn run_vec_of_pipelines(
    pipelines: Vec<String>,
    redirect_stdin: bool,
) -> Result<(), Box<dyn Error>> {
    let mut syncer = crate::EnvironmentSyncer::new();
    let mut context = create_default_context(&mut syncer, false)?;

    let _ = crate::load_plugins(&mut context);

    let cc = context.ctrl_c.clone();

    ctrlc::set_handler(move || {
        cc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if context.ctrl_c.load(Ordering::SeqCst) {
        context.ctrl_c.store(false, Ordering::SeqCst);
    }

    // before we start up, let's run our startup commands
    if let Ok(config) = crate::data::config::config(Tag::unknown()) {
        if let Some(commands) = config.get("startup") {
            match commands {
                Value {
                    value: UntaggedValue::Table(pipelines),
                    ..
                } => {
                    for pipeline in pipelines {
                        if let Ok(pipeline_string) = pipeline.as_string() {
                            let _ = run_pipeline_standalone(
                                pipeline_string,
                                false,
                                &mut context,
                                false,
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    println!("warning: expected a table of pipeline strings as startup commands");
                }
            }
        }
    }

    for pipeline in pipelines {
        run_pipeline_standalone(pipeline, redirect_stdin, &mut context, true).await?;
    }
    Ok(())
}

pub async fn run_pipeline_standalone(
    pipeline: String,
    redirect_stdin: bool,
    context: &mut Context,
    exit_on_error: bool,
) -> Result<(), Box<dyn Error>> {
    let line = process_line(Ok(pipeline), context, redirect_stdin, false).await;

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
            context.with_host(|host| {
                print_err(err, host, &Text::from(line.clone()));
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

/// The entry point for the CLI. Will register all known internal commands, load experimental commands, load plugins, then prepare the prompt and line reader for input.
pub async fn cli() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    let mut syncer = crate::EnvironmentSyncer::new();
    let mut context = create_default_context(&mut syncer, true)?;

    let _ = load_plugins(&mut context);

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

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    // we are ok if history does not exist
    let _ = rl.load_history(&History::path());

    let cc = context.ctrl_c.clone();
    ctrlc::set_handler(move || {
        cc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    let mut ctrlcbreak = false;

    // before we start up, let's run our startup commands
    if let Ok(config) = crate::data::config::config(Tag::unknown()) {
        if let Some(commands) = config.get("startup") {
            match commands {
                Value {
                    value: UntaggedValue::Table(pipelines),
                    ..
                } => {
                    for pipeline in pipelines {
                        if let Ok(pipeline_string) = pipeline.as_string() {
                            let _ = run_pipeline_standalone(
                                pipeline_string,
                                false,
                                &mut context,
                                false,
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    println!("warning: expected a table of pipeline strings as startup commands");
                }
            }
        }
    }

    loop {
        if context.ctrl_c.load(Ordering::SeqCst) {
            context.ctrl_c.store(false, Ordering::SeqCst);
            continue;
        }

        let cwd = context.shell_manager.path();

        rl.set_helper(Some(crate::shell::Helper::new(context.clone())));

        let edit_mode = config::config(Tag::unknown())?
            .get("edit_mode")
            .map(|s| match s.value.expect_string() {
                "vi" => EditMode::Vi,
                "emacs" => EditMode::Emacs,
                _ => EditMode::Emacs,
            })
            .unwrap_or(EditMode::Emacs);

        rl.set_edit_mode(edit_mode);

        let key_timeout = config::config(Tag::unknown())?
            .get("key_timeout")
            .map(|s| s.value.expect_int())
            .unwrap_or(1);

        rl.set_keyseq_timeout(key_timeout as i32);

        let completion_mode = config::config(Tag::unknown())?
            .get("completion_mode")
            .map(|s| match s.value.expect_string() {
                "list" => CompletionType::List,
                "circular" => CompletionType::Circular,
                _ => DEFAULT_COMPLETION_MODE,
            })
            .unwrap_or(DEFAULT_COMPLETION_MODE);

        rl.set_completion_type(completion_mode);

        let colored_prompt = {
            #[cfg(feature = "starship-prompt")]
            {
                std::env::set_var("STARSHIP_SHELL", "");
                let mut starship_context =
                    starship::context::Context::new_with_dir(clap::ArgMatches::default(), cwd);

                match starship_context.config.config {
                    None => {
                        starship_context.config.config = create_default_starship_config();
                    }
                    Some(toml::Value::Table(t)) if t.is_empty() => {
                        starship_context.config.config = create_default_starship_config();
                    }
                    _ => {}
                };
                starship::print::get_prompt(starship_context)
            }
            #[cfg(not(feature = "starship-prompt"))]
            {
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

        let line = process_line(readline, &mut context, false, true).await;

        // Check the config to see if we need to update the path
        // TODO: make sure config is cached so we don't path this load every call
        // FIXME: we probably want to be a bit more graceful if we can't set the environment
        syncer.reload();
        syncer.sync_env_vars(&mut context);
        syncer.sync_path_vars(&mut context);

        match line {
            LineResult::Success(line) => {
                rl.add_history_entry(line.clone());
                let _ = rl.save_history(&History::path());
                context.maybe_print_errors(Text::from(line));
            }

            LineResult::Error(line, err) => {
                rl.add_history_entry(line.clone());
                let _ = rl.save_history(&History::path());

                context.with_host(|host| {
                    print_err(err, host, &Text::from(line.clone()));
                });

                context.maybe_print_errors(Text::from(line.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| match s.value.expect_string() {
                        "true" => true,
                        _ => false,
                    })
                    .unwrap_or(false); // default behavior is to allow CTRL-C spamming similar to other shells

                if !config_ctrlc_exit {
                    continue;
                }

                if ctrlcbreak {
                    let _ = rl.save_history(&History::path());
                    std::process::exit(0);
                } else {
                    context.with_host(|host| host.stdout("CTRL-C pressed (again to quit)"));
                    ctrlcbreak = true;
                    continue;
                }
            }

            LineResult::Break => {
                break;
            }
        }
        ctrlcbreak = false;
    }

    // we are ok if we can not save history
    let _ = rl.save_history(&History::path());

    Ok(())
}

fn chomp_newline(s: &str) -> &str {
    if s.ends_with('\n') {
        &s[..s.len() - 1]
    } else {
        s
    }
}

enum LineResult {
    Success(String),
    Error(String, ShellError),
    CtrlC,
    Break,
}

/// Process the line by parsing the text to turn it into commands, classify those commands so that we understand what is being called in the pipeline, and then run this pipeline
async fn process_line(
    readline: Result<String, ReadlineError>,
    ctx: &mut Context,
    redirect_stdin: bool,
    cli_mode: bool,
) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);

            let result = match nu_parser::lite_parse(&line, 0) {
                Err(err) => {
                    return LineResult::Error(line.to_string(), err.into());
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let mut classified_block = nu_parser::classify_block(&result, ctx.registry());

            debug!("{:#?}", classified_block);
            //println!("{:#?}", pipeline);

            if let Some(failure) = classified_block.failed {
                return LineResult::Error(line.to_string(), failure.into());
            }

            // There's a special case to check before we process the pipeline:
            // If we're giving a path by itself
            // ...and it's not a command in the path
            // ...and it doesn't have any arguments
            // ...and we're in the CLI
            // ...then change to this directory
            if cli_mode
                && classified_block.block.block.len() == 1
                && classified_block.block.block[0].list.len() == 1
            {
                if let ClassifiedCommand::Internal(InternalCommand {
                    ref name, ref args, ..
                }) = classified_block.block.block[0].list[0]
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
                        && which::which(&name).is_err()
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
                let stream = FramedRead::new(file, MaybeTextCodec).map(|line| {
                    if let Ok(line) = line {
                        match line {
                            StringOrBinary::String(s) => Ok(Value {
                                value: UntaggedValue::Primitive(Primitive::String(s)),
                                tag: Tag::unknown(),
                            }),
                            StringOrBinary::Binary(b) => Ok(Value {
                                value: UntaggedValue::Primitive(Primitive::Binary(
                                    b.into_iter().collect(),
                                )),
                                tag: Tag::unknown(),
                            }),
                        }
                    } else {
                        panic!("Internal error: could not read lines of text from stdin")
                    }
                });
                stream.to_input_stream()
            } else {
                InputStream::empty()
            };

            classified_block.block.expand_it_usage();

            trace!("{:#?}", classified_block);
            let env = ctx.get_env();
            match run_block(&classified_block.block, ctx, input_stream, &Scope::env(env)).await {
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
                        registry: ctx.registry.clone(),
                        name: Tag::unknown(),
                    };

                    if let Ok(mut output_stream) = crate::commands::autoview::autoview(context) {
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
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::Break,
        Err(err) => {
            outln!("Error: {:?}", err);
            LineResult::Break
        }
    }
}

pub fn print_err(err: ShellError, host: &dyn Host, source: &Text) {
    if let Some(diag) = err.into_diagnostic() {
        let writer = host.err_termcolor();
        let mut source = source.to_string();
        source.push_str(" ");
        let files = nu_parser::Files::new(source);
        let _ = std::panic::catch_unwind(move || {
            let _ = language_reporting::emit(
                &mut writer.lock(),
                &files,
                &diag,
                &language_reporting::DefaultConfig,
            );
        });
    }
}

#[cfg(test)]
mod tests {
    #[quickcheck]
    fn quickcheck_parse(data: String) -> bool {
        if let Ok(lite_block) = nu_parser::lite_parse(&data, 0) {
            let context = crate::context::Context::basic().unwrap();
            let _ = nu_parser::classify_block(&lite_block, context.registry());
        }
        true
    }
}
