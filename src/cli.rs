use crate::commands::classified::{
    ClassifiedCommand, ClassifiedInputStream, ClassifiedPipeline, ExternalCommand, InternalCommand,
    StreamNext,
};
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::whole_stream_command;
use crate::context::Context;
use crate::data::config;
use crate::data::Value;
pub(crate) use crate::errors::ShellError;
use crate::fuzzysearch::{interactive_fuzzy_search, SelectionResult};
use crate::git::current_branch;
use crate::parser::registry::Signature;
use crate::parser::{
    hir,
    hir::syntax_shape::{CommandHeadShape, CommandSignature, ExpandSyntax},
    hir::{expand_external_tokens::expand_external_tokens, tokens_iterator::TokensIterator},
    parse_command_tail, Pipeline, PipelineElement, TokenNode,
};
use crate::prelude::*;

use log::{debug, trace};
use rustyline::error::ReadlineError;
use rustyline::{self, config::Configurer, config::EditMode, ColorMode, Config, Editor};
use std::env;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> MaybeOwned<'_, T> {
    pub fn borrow(&self) -> &T {
        match self {
            MaybeOwned::Owned(v) => v,
            MaybeOwned::Borrowed(v) => v,
        }
    }
}

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
    stdin.write(format!("{}\n", request_raw).as_bytes())?;
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

                        if let Some(_) = context.get_command(&name) {
                            trace!("plugin {:?} already loaded.", &name);
                        } else {
                            if params.is_filter {
                                context.add_commands(vec![whole_stream_command(
                                    PluginCommand::new(name, fname, params),
                                )]);
                            } else {
                                context.add_commands(vec![whole_stream_command(PluginSink::new(
                                    name, fname, params,
                                ))]);
                            };
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                Err(e) => {
                    trace!("incompatible plugin {:?}", input);
                    Err(ShellError::string(format!("Error: {:?}", e)))
                }
            }
        }
        Err(e) => Err(ShellError::string(format!("Error: {:?}", e))),
    };

    let _ = child.wait();

    result
}

fn search_paths() -> Vec<std::path::PathBuf> {
    let mut search_paths = Vec::new();

    match env::var_os("PATH") {
        Some(paths) => {
            search_paths = env::split_paths(&paths).collect::<Vec<_>>();
        }
        None => println!("PATH is not defined in the environment."),
    }

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
            .unwrap_or(PathBuf::from(FNAME))
    }
}

pub async fn cli() -> Result<(), Box<dyn Error>> {
    let mut context = Context::basic()?;

    {
        use crate::commands::*;

        context.add_commands(vec![
            whole_stream_command(PWD),
            whole_stream_command(LS),
            whole_stream_command(CD),
            whole_stream_command(Size),
            whole_stream_command(Nth),
            whole_stream_command(Next),
            whole_stream_command(Previous),
            whole_stream_command(Debug),
            whole_stream_command(Lines),
            whole_stream_command(Shells),
            whole_stream_command(SplitColumn),
            whole_stream_command(SplitRow),
            whole_stream_command(Lines),
            whole_stream_command(Reject),
            whole_stream_command(Reverse),
            whole_stream_command(Trim),
            whole_stream_command(ToBSON),
            whole_stream_command(ToCSV),
            whole_stream_command(ToJSON),
            whole_stream_command(ToSQLite),
            whole_stream_command(ToDB),
            whole_stream_command(ToTOML),
            whole_stream_command(ToTSV),
            whole_stream_command(ToURL),
            whole_stream_command(ToYAML),
            whole_stream_command(SortBy),
            whole_stream_command(Tags),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(Env),
            whole_stream_command(FromCSV),
            whole_stream_command(FromTSV),
            whole_stream_command(FromINI),
            whole_stream_command(FromBSON),
            whole_stream_command(FromJSON),
            whole_stream_command(FromDB),
            whole_stream_command(FromSQLite),
            whole_stream_command(FromTOML),
            whole_stream_command(FromURL),
            whole_stream_command(FromXML),
            whole_stream_command(FromYAML),
            whole_stream_command(FromYML),
            whole_stream_command(Pick),
            whole_stream_command(Get),
            per_item_command(Remove),
            per_item_command(Fetch),
            per_item_command(Open),
            per_item_command(Post),
            per_item_command(Where),
            per_item_command(Echo),
            whole_stream_command(Config),
            whole_stream_command(SkipWhile),
            per_item_command(Enter),
            per_item_command(Help),
            whole_stream_command(Exit),
            whole_stream_command(Autoview),
            whole_stream_command(Pivot),
            per_item_command(Cpy),
            whole_stream_command(Date),
            per_item_command(Mkdir),
            per_item_command(Move),
            whole_stream_command(Save),
            whole_stream_command(Table),
            whole_stream_command(Version),
            whole_stream_command(Which),
        ]);

        #[cfg(feature = "clipboard")]
        {
            context.add_commands(vec![whole_stream_command(
                crate::commands::clip::clipboard::Clip,
            )]);
        }
    }
    let _ = load_plugins(&mut context);

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let mut rl: Editor<_> = Editor::with_config(config);

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    // we are ok if history does not exist
    let _ = rl.load_history(&History::path());

    let ctrl_c = Arc::new(AtomicBool::new(false));
    let cc = ctrl_c.clone();
    ctrlc::set_handler(move || {
        cc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    let mut ctrlcbreak = false;
    loop {
        if ctrl_c.load(Ordering::SeqCst) {
            ctrl_c.store(false, Ordering::SeqCst);
            continue;
        }

        let cwd = context.shell_manager.path();

        rl.set_helper(Some(crate::shell::Helper::new(
            context.shell_manager.clone(),
        )));

        let edit_mode = config::config(Tag::unknown())?
            .get("edit_mode")
            .map(|s| match s.as_string().unwrap().as_ref() {
                "vi" => EditMode::Vi,
                "emacs" => EditMode::Emacs,
                _ => EditMode::Emacs,
            })
            .unwrap_or(EditMode::Emacs);

        rl.set_edit_mode(edit_mode);

        // Register Ctrl-r for history fuzzy search
        // rustyline doesn't support custom commands, so we override Ctrl-D (EOF)
        // https://github.com/nushell/nushell/issues/689
        #[cfg(all(not(windows), feature = "crossterm"))]
        rl.bind_sequence(rustyline::KeyPress::Ctrl('R'), rustyline::Cmd::EndOfFile);
        // Redefine Ctrl-D to same command as Ctrl-C
        rl.bind_sequence(rustyline::KeyPress::Ctrl('D'), rustyline::Cmd::Interrupt);

        let prompt = &format!(
            "{}{}> ",
            cwd,
            match current_branch() {
                Some(s) => format!("({})", s),
                None => "".to_string(),
            }
        );
        let mut initial_command = Some(String::new());
        let mut readline = Err(ReadlineError::Eof);
        while let Some(ref cmd) = initial_command {
            readline = rl.readline_with_initial(prompt, (&cmd, ""));
            if let Err(ReadlineError::Eof) = &readline {
                // Fuzzy search in history
                let lines = rl.history().iter().rev().map(|s| s.as_str()).collect();
                let selection = interactive_fuzzy_search(&lines, 5); // Clears last line with prompt
                match selection {
                    SelectionResult::Selected(line) => {
                        println!("{}{}", &prompt, &line); // TODO: colorize prompt
                        readline = Ok(line.clone());
                        initial_command = None;
                    }
                    SelectionResult::Edit(line) => {
                        initial_command = Some(line);
                    }
                    SelectionResult::NoSelection => {
                        readline = Ok("".to_string());
                        initial_command = None;
                    }
                }
            } else {
                initial_command = None;
            }
        }

        match process_line(readline, &mut context).await {
            LineResult::Success(line) => {
                rl.add_history_entry(line.clone());
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| match s.as_string().unwrap().as_ref() {
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

            LineResult::Error(line, err) => {
                rl.add_history_entry(line.clone());

                context.with_host(|host| {
                    print_err(err, host, &Text::from(line));
                })
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

async fn process_line(readline: Result<String, ReadlineError>, ctx: &mut Context) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);

            let result = match crate::parser::parse(&line, uuid::Uuid::new_v4()) {
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

            match pipeline.commands.last() {
                Some(ClassifiedCommand::External(_)) => {}
                _ => pipeline
                    .commands
                    .push(ClassifiedCommand::Internal(InternalCommand {
                        name: "autoview".to_string(),
                        name_tag: Tag::unknown(),
                        args: hir::Call::new(
                            Box::new(hir::Expression::synthetic_string("autoview")),
                            None,
                            None,
                        ),
                    })),
            }

            let mut input = ClassifiedInputStream::new();

            let mut iter = pipeline.commands.into_iter().peekable();
            let mut is_first_command = true;

            loop {
                let item: Option<ClassifiedCommand> = iter.next();
                let next: Option<&ClassifiedCommand> = iter.peek();

                input = match (item, next) {
                    (None, _) => break,

                    (Some(ClassifiedCommand::Dynamic(_)), _)
                    | (_, Some(ClassifiedCommand::Dynamic(_))) => {
                        return LineResult::Error(
                            line.to_string(),
                            ShellError::unimplemented("Dynamic commands"),
                        )
                    }

                    (Some(ClassifiedCommand::Expr(_)), _) => {
                        return LineResult::Error(
                            line.to_string(),
                            ShellError::unimplemented("Expression-only commands"),
                        )
                    }

                    (_, Some(ClassifiedCommand::Expr(_))) => {
                        return LineResult::Error(
                            line.to_string(),
                            ShellError::unimplemented("Expression-only commands"),
                        )
                    }

                    (
                        Some(ClassifiedCommand::Internal(left)),
                        Some(ClassifiedCommand::External(_)),
                    ) => match left
                        .run(ctx, input, Text::from(line), is_first_command)
                        .await
                    {
                        Ok(val) => ClassifiedInputStream::from_input_stream(val),
                        Err(err) => return LineResult::Error(line.to_string(), err),
                    },

                    (Some(ClassifiedCommand::Internal(left)), Some(_)) => {
                        match left
                            .run(ctx, input, Text::from(line), is_first_command)
                            .await
                        {
                            Ok(val) => ClassifiedInputStream::from_input_stream(val),
                            Err(err) => return LineResult::Error(line.to_string(), err),
                        }
                    }

                    (Some(ClassifiedCommand::Internal(left)), None) => {
                        match left
                            .run(ctx, input, Text::from(line), is_first_command)
                            .await
                        {
                            Ok(val) => ClassifiedInputStream::from_input_stream(val),
                            Err(err) => return LineResult::Error(line.to_string(), err),
                        }
                    }

                    (
                        Some(ClassifiedCommand::External(left)),
                        Some(ClassifiedCommand::External(_)),
                    ) => match left.run(ctx, input, StreamNext::External).await {
                        Ok(val) => val,
                        Err(err) => return LineResult::Error(line.to_string(), err),
                    },

                    (Some(ClassifiedCommand::External(left)), Some(_)) => {
                        match left.run(ctx, input, StreamNext::Internal).await {
                            Ok(val) => val,
                            Err(err) => return LineResult::Error(line.to_string(), err),
                        }
                    }

                    (Some(ClassifiedCommand::External(left)), None) => {
                        match left.run(ctx, input, StreamNext::Last).await {
                            Ok(val) => val,
                            Err(err) => return LineResult::Error(line.to_string(), err),
                        }
                    }
                };

                is_first_command = false;
            }

            LineResult::Success(line.to_string())
        }
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::Break,
        Err(err) => {
            println!("Error: {:?}", err);
            LineResult::Break
        }
    }
}

fn classify_pipeline(
    pipeline: &TokenNode,
    context: &Context,
    source: &Text,
) -> Result<ClassifiedPipeline, ShellError> {
    let pipeline = pipeline.as_pipeline()?;

    let Pipeline { parts, .. } = pipeline;

    let commands: Result<Vec<_>, ShellError> = parts
        .iter()
        .map(|item| classify_command(&item, context, &source))
        .collect();

    Ok(ClassifiedPipeline {
        commands: commands?,
    })
}

fn classify_command(
    command: &Tagged<PipelineElement>,
    context: &Context,
    source: &Text,
) -> Result<ClassifiedCommand, ShellError> {
    let mut iterator = TokensIterator::new(&command.tokens.item, command.tag, true);

    let head = CommandHeadShape
        .expand_syntax(&mut iterator, &context.expand_context(source, command.tag))?;

    match &head {
        CommandSignature::Expression(_) => Err(ShellError::syntax_error(
            "Unexpected expression in command position".tagged(command.tag),
        )),

        // If the command starts with `^`, treat it as an external command no matter what
        CommandSignature::External(name) => {
            let name_str = name.slice(source);

            external_command(&mut iterator, source, name_str.tagged(name))
        }

        CommandSignature::LiteralExternal { outer, inner } => {
            let name_str = inner.slice(source);

            external_command(&mut iterator, source, name_str.tagged(outer))
        }

        CommandSignature::Internal(command) => {
            let tail = parse_command_tail(
                &command.signature(),
                &context.expand_context(source, command.tag),
                &mut iterator,
                command.tag,
            )?;

            let (positional, named) = match tail {
                None => (None, None),
                Some((positional, named)) => (positional, named),
            };

            let call = hir::Call {
                head: Box::new(head.to_expression()),
                positional,
                named,
            };

            Ok(ClassifiedCommand::Internal(InternalCommand::new(
                command.name().to_string(),
                command.tag,
                call,
            )))
        }
    }
}

// Classify this command as an external command, which doesn't give special meaning
// to nu syntactic constructs, and passes all arguments to the external command as
// strings.
pub(crate) fn external_command(
    tokens: &mut TokensIterator,
    source: &Text,
    name: Tagged<&str>,
) -> Result<ClassifiedCommand, ShellError> {
    let arg_list_strings = expand_external_tokens(tokens, source)?;

    Ok(ClassifiedCommand::External(ExternalCommand {
        name: name.to_string(),
        name_tag: name.tag(),
        args: arg_list_strings,
    }))
}

pub fn print_err(err: ShellError, host: &dyn Host, source: &Text) {
    let diag = err.to_diagnostic();

    let writer = host.err_termcolor();
    let mut source = source.to_string();
    source.push_str(" ");
    let files = crate::parser::Files::new(source);
    let _ = std::panic::catch_unwind(move || {
        let _ = language_reporting::emit(
            &mut writer.lock(),
            &files,
            &diag,
            &language_reporting::DefaultConfig,
        );
    });
}
