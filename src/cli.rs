use crate::commands::classified::pipeline::run_pipeline;
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::whole_stream_command;
use crate::context::Context;
use crate::data::config;
#[cfg(not(feature = "starship-prompt"))]
use crate::git::current_branch;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_parser::{
    expand_syntax, hir, ClassifiedCommand, ClassifiedPipeline, InternalCommand, PipelineShape,
    TokenNode, TokensIterator,
};
use nu_protocol::{Signature, UntaggedValue, Value};

use log::{debug, log_enabled, trace};
use rustyline::error::ReadlineError;
use rustyline::{
    self, config::Configurer, config::EditMode, At, Cmd, ColorMode, Config, Editor, KeyPress,
    Movement, Word,
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

                        if context.get_command(&name)?.is_some() {
                            trace!("plugin {:?} already loaded.", &name);
                        } else if params.is_filter {
                            context.add_commands(vec![whole_stream_command(
                                PluginCommand::new(name, fname, params),
                            )])?;
                        } else {
                            context.add_commands(vec![whole_stream_command(PluginSink::new(
                                name, fname, params,
                            ))])?;
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
    let mut search_paths = Vec::new();

    #[cfg(debug_assertions)]
    {
        // Use our debug plugins in debug mode
        let mut path = std::path::PathBuf::from(".");
        path.push("target");
        path.push("debug");

        if path.exists() {
            search_paths.push(path);
        }
    }

    #[cfg(not(debug_assertions))]
    {
        use std::env;

        match env::var_os("PATH") {
            Some(paths) => {
                search_paths = env::split_paths(&paths).collect::<Vec<_>>();
            }
            None => println!("PATH is not defined in the environment."),
        }

        // Use our release plugins in release mode
        let mut path = std::path::PathBuf::from(".");
        path.push("target");
        path.push("release");

        if path.exists() {
            search_paths.push(path);
        }
    }

    // permit Nu finding and picking up development plugins
    // if there are any first.
    search_paths.reverse();
    search_paths
}

fn load_plugins(context: &mut Context) -> Result<(), ShellError> {
    let opts = glob::MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    set_env_from_config()?;

    for path in search_paths() {
        let mut pattern = path.to_path_buf();

        pattern.push(std::path::Path::new("nu_plugin_[a-z]*"));

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
                                .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '.')
                        }

                        #[cfg(not(windows))]
                        {
                            bin_name
                                .chars()
                                .all(|c| c.is_ascii_alphabetic() || c == '_')
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

pub async fn cli() -> Result<(), Box<dyn Error>> {
    let mut context = Context::basic()?;

    {
        use crate::commands::*;

        context.add_commands(vec![
            // System/file operations
            whole_stream_command(Pwd),
            whole_stream_command(Ls),
            whole_stream_command(Cd),
            whole_stream_command(Env),
            per_item_command(Remove),
            per_item_command(Open),
            whole_stream_command(Config),
            per_item_command(Help),
            per_item_command(History),
            whole_stream_command(Save),
            per_item_command(Cpy),
            whole_stream_command(Date),
            per_item_command(Mkdir),
            per_item_command(Move),
            whole_stream_command(Version),
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
            whole_stream_command(Uniq),
            // Table manipulation
            whole_stream_command(Wrap),
            whole_stream_command(Pivot),
            // Data processing
            whole_stream_command(Histogram),
            // File format output
            whole_stream_command(ToBSON),
            whole_stream_command(ToCSV),
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
        ])?;

        cfg_if::cfg_if! {
            if #[cfg(data_processing_primitives)] {
                context.add_commands(vec![
                whole_stream_command(ReduceBy),
                whole_stream_command(EvaluateBy),
                whole_stream_command(TSortBy),
                whole_stream_command(MapMaxBy),
                ])?;
            }
        }

        #[cfg(feature = "clipboard")]
        {
            context.add_commands(vec![whole_stream_command(
                crate::commands::clip::clipboard::Clip,
            )])?;
        }
    }

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

        let cwd = context.shell_manager.path()?;

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

        let colored_prompt = {
            #[cfg(feature = "starship-prompt")]
            {
                std::env::set_var("STARSHIP_SHELL", "");
                starship::print::get_prompt(starship::context::Context::new_with_dir(
                    clap::ArgMatches::default(),
                    cwd,
                ))
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

        let line = process_line(readline, &mut context).await;

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
                })?;

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
                    context.with_host(|host| host.stdout("CTRL-C pressed (again to quit)"))?;
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

fn set_env_from_config() -> Result<(), ShellError> {
    let config = crate::data::config::read(Tag::unknown(), &None)?;

    if config.contains_key("env") {
        // Clear the existing vars, we're about to replace them
        for (key, _value) in std::env::vars() {
            std::env::remove_var(key);
        }

        let value = config.get("env");

        if let Some(Value {
            value: UntaggedValue::Row(r),
            ..
        }) = value
        {
            for (k, v) in &r.entries {
                if let Ok(value_string) = v.as_string() {
                    std::env::set_var(k, value_string);
                }
            }
        }
    }

    if config.contains_key("path") {
        // Override the path with what they give us from config
        let value = config.get("path");

        if let Some(Value {
            value: UntaggedValue::Table(table),
            ..
        }) = value
        {
            let mut paths = vec![];

            for val in table {
                let path_str = val.as_string();

                if let Ok(path_str) = path_str {
                    paths.push(PathBuf::from(path_str));
                }
            }

            let path_os_string = std::env::join_paths(&paths);
            if let Ok(path_os_string) = path_os_string {
                std::env::set_var("PATH", path_os_string);
            }
        }
    }
    Ok(())
}

enum LineResult {
    Success(String),
    Error(String, ShellError),
    CtrlC,
    Break,
}

async fn process_line(readline: Result<String, ReadlineError>, ctx: &mut Context) -> LineResult {
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

            let mut pipeline = match classify_pipeline(&result, ctx, &Text::from(line)) {
                Ok(pipeline) => pipeline,
                Err(err) => return LineResult::Error(line.to_string(), err),
            };

            match pipeline.commands.list.last() {
                Some(ClassifiedCommand::External(_)) => {}
                _ => pipeline
                    .commands
                    .list
                    .push(ClassifiedCommand::Internal(InternalCommand {
                        name: "autoview".to_string(),
                        name_tag: Tag::unknown(),
                        args: hir::Call::new(
                            Box::new(hir::Expression::synthetic_string("autoview")),
                            None,
                            None,
                            Span::unknown(),
                        ),
                    })),
            }

            // Check the config to see if we need to update the path
            // TODO: make sure config is cached so we don't path this load every call
            // FIXME: we probably want to be a bit more graceful if we can't set the environment
            if let Err(err) = set_env_from_config() {
                return LineResult::Error(line.to_string(), err);
            }

            match run_pipeline(pipeline, ctx, None, line).await {
                Ok(_) => LineResult::Success(line.to_string()),
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
    pipeline: &TokenNode,
    context: &Context,
    source: &Text,
) -> Result<ClassifiedPipeline, ShellError> {
    let pipeline_list = vec![pipeline.clone()];
    let mut iterator = TokensIterator::all(&pipeline_list, source.clone(), pipeline.span());

    let result = expand_syntax(
        &PipelineShape,
        &mut iterator,
        &context.expand_context(source)?,
    )
    .map_err(|err| err.into());

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
