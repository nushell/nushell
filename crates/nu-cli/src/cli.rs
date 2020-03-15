use crate::commands::classified::external::{MaybeTextCodec, StringOrBinary};
use crate::commands::classified::pipeline::run_pipeline;
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::whole_stream_command;
use crate::context::Context;
#[cfg(not(feature = "starship-prompt"))]
use crate::git::current_branch;
use crate::prelude::*;
use futures_codec::FramedRead;

use nu_errors::ShellError;
use nu_parser::{ClassifiedPipeline, PipelineShape, SpannedToken, TokensIterator};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

use log::{debug, log_enabled, trace};
use rustyline::error::ReadlineError;
use rustyline::{
    self, config::Configurer, config::EditMode, At, Cmd, ColorMode, CompletionType, Config, Editor,
    KeyPress, Movement, Word,
};
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

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

        match glob::glob_with(&pattern.to_string_lossy(), opts) {
            Err(_) => {}
            Ok(binaries) => {
                for bin in binaries.filter_map(Result::ok) {
                    if !bin.is_file() {
                        continue;
                    }

                    let bin_name = {
                        if let Some(name) = bin.file_name() {
                            match name.to_str() {
                                Some(raw) => raw,
                                None => continue,
                            }
                        } else {
                            continue;
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
                        trace!("Trying {:?}", bin.display());

                        // we are ok if this plugin load fails
                        let _ = load_plugin(&bin, context);
                    }
                }
            }
        }
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
    syncer: &mut crate::env::environment_syncer::EnvironmentSyncer,
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
            per_item_command(Ls),
            per_item_command(Du),
            whole_stream_command(Cd),
            per_item_command(Remove),
            per_item_command(Open),
            whole_stream_command(Config),
            per_item_command(Help),
            per_item_command(History),
            whole_stream_command(Save),
            per_item_command(Touch),
            per_item_command(Cpy),
            whole_stream_command(Date),
            per_item_command(Calc),
            per_item_command(Mkdir),
            per_item_command(Move),
            per_item_command(Kill),
            whole_stream_command(Version),
            whole_stream_command(Clear),
            whole_stream_command(What),
            whole_stream_command(Which),
            whole_stream_command(Debug),
            // Statistics
            whole_stream_command(Size),
            whole_stream_command(Count),
            // Metadata
            whole_stream_command(Tags),
            // Shells
            whole_stream_command(Next),
            whole_stream_command(Previous),
            whole_stream_command(Shells),
            per_item_command(Enter),
            whole_stream_command(Exit),
            // Viewers
            whole_stream_command(Autoview),
            whole_stream_command(Table),
            // Text manipulation
            whole_stream_command(SplitColumn),
            whole_stream_command(SplitRow),
            whole_stream_command(Lines),
            whole_stream_command(Trim),
            per_item_command(Echo),
            per_item_command(Parse),
            // Column manipulation
            whole_stream_command(Reject),
            whole_stream_command(Pick),
            whole_stream_command(Get),
            per_item_command(Edit),
            per_item_command(Insert),
            whole_stream_command(SplitBy),
            // Row manipulation
            whole_stream_command(Reverse),
            whole_stream_command(Append),
            whole_stream_command(Prepend),
            whole_stream_command(SortBy),
            whole_stream_command(GroupBy),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(Skip),
            whole_stream_command(Nth),
            per_item_command(Format),
            per_item_command(Where),
            whole_stream_command(Compact),
            whole_stream_command(Default),
            whole_stream_command(SkipWhile),
            whole_stream_command(Range),
            whole_stream_command(Rename),
            whole_stream_command(Uniq),
            // Table manipulation
            whole_stream_command(Shuffle),
            whole_stream_command(Wrap),
            whole_stream_command(Pivot),
            // Data processing
            whole_stream_command(Histogram),
            // File format output
            whole_stream_command(ToBSON),
            whole_stream_command(ToCSV),
            whole_stream_command(ToHTML),
            whole_stream_command(ToJSON),
            whole_stream_command(ToSQLite),
            whole_stream_command(ToDB),
            whole_stream_command(ToTOML),
            whole_stream_command(ToTSV),
            whole_stream_command(ToURL),
            whole_stream_command(ToYAML),
            // File format input
            whole_stream_command(FromCSV),
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

pub async fn run_pipeline_standalone(
    pipeline: String,
    redirect_stdin: bool,
    context: &mut Context,
) -> Result<(), Box<dyn Error>> {
    let line = process_line(Ok(pipeline), context, redirect_stdin).await;

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
            if error_code != 0 {
                std::process::exit(error_code);
            }
        }

        LineResult::Error(line, err) => {
            context.with_host(|host| {
                print_err(err, host, &Text::from(line.clone()));
            });

            context.maybe_print_errors(Text::from(line));
            std::process::exit(1);
        }

        _ => {}
    }

    Ok(())
}

/// The entry point for the CLI. Will register all known internal commands, load experimental commands, load plugins, then prepare the prompt and line reader for input.
pub async fn cli() -> Result<(), Box<dyn Error>> {
    let mut syncer = crate::env::environment_syncer::EnvironmentSyncer::new();
    let mut context = create_default_context(&mut syncer)?;

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

        let completion_mode = config::config(Tag::unknown())?
            .get("completion_mode")
            .map(|s| match s.value.expect_string() {
                "list" => CompletionType::List,
                "circular" => CompletionType::Circular,
                _ => CompletionType::Circular,
            })
            .unwrap_or(CompletionType::Circular);

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

        let line = process_line(readline, &mut context, false).await;

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
) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);

            let result = match nu_parser::parse(&line) {
                Err(err) => {
                    return LineResult::Error(line.to_string(), err);
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let pipeline = classify_pipeline(&result, ctx, &Text::from(line));

            if let Some(failure) = pipeline.failed {
                return LineResult::Error(line.to_string(), failure.into());
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
                Some(stream.to_input_stream())
            } else {
                None
            };

            match run_pipeline(pipeline, ctx, input_stream, line).await {
                Ok(Some(input)) => {
                    // Running a pipeline gives us back a stream that we can then
                    // work through. At the top level, we just want to pull on the
                    // values to compute them.
                    use futures::stream::TryStreamExt;

                    let context = RunnableContext {
                        input,
                        shell_manager: ctx.shell_manager.clone(),
                        host: ctx.host.clone(),
                        ctrl_c: ctx.ctrl_c.clone(),
                        commands: ctx.registry.clone(),
                        name: Tag::unknown(),
                        source: Text::from(String::new()),
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
                                _ => {
                                    break;
                                }
                            }
                        }
                    }

                    LineResult::Success(line.to_string())
                }
                Ok(None) => LineResult::Success(line.to_string()),
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

pub fn classify_pipeline(
    pipeline: &SpannedToken,
    context: &Context,
    source: &Text,
) -> ClassifiedPipeline {
    let pipeline_list = vec![pipeline.clone()];
    let expand_context = context.expand_context(source);
    let mut iterator = TokensIterator::new(&pipeline_list, expand_context, pipeline.span());

    let result = iterator.expand_infallible(PipelineShape);

    if log_enabled!(target: "nu::expand_syntax", log::Level::Debug) {
        outln!("");
        let _ = ptree::print_tree(&iterator.expand_tracer().print(source.clone()));
        outln!("");
    }

    result
}

pub fn print_err(err: ShellError, host: &dyn Host, source: &Text) {
    let diag = err.into_diagnostic();

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
