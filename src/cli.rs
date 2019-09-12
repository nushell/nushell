use crate::commands::autoview;
use crate::commands::classified::{
    ClassifiedCommand, ClassifiedInputStream, ClassifiedPipeline, ExternalCommand, InternalCommand,
    StreamNext,
};
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::whole_stream_command;
use crate::context::Context;
use crate::data::Value;
pub(crate) use crate::errors::ShellError;
use crate::git::current_branch;
use crate::parser::registry::Signature;
use crate::parser::{hir, CallNode, Pipeline, PipelineElement, TokenNode};
use crate::prelude::*;

use log::{debug, trace};
use rustyline::error::ReadlineError;
use rustyline::{self, config::Configurer, config::EditMode, ColorMode, Config, Editor};
use std::env;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::iter::Iterator;
use std::sync::atomic::{AtomicBool, Ordering};

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

            let response = serde_json::from_str::<JsonRpc<Result<Signature, ShellError>>>(&input);
            match response {
                Ok(jrpc) => match jrpc.params {
                    Ok(params) => {
                        let fname = path.to_string_lossy();

                        trace!("processing {:?}", params);

                        if params.is_filter {
                            let fname = fname.to_string();
                            let name = params.name.clone();
                            context.add_commands(vec![whole_stream_command(PluginCommand::new(
                                name, fname, params,
                            ))]);
                            Ok(())
                        } else {
                            let fname = fname.to_string();
                            let name = params.name.clone();
                            context.add_commands(vec![whole_stream_command(PluginSink::new(
                                name, fname, params,
                            ))]);
                            Ok(())
                        }
                    }
                    Err(e) => Err(e),
                },
                Err(e) => Err(ShellError::string(format!("Error: {:?}", e))),
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
        search_paths.push(path);
    }

    #[cfg(not(debug_assertions))]
    {
        // Use our release plugins in release mode
        let mut path = std::path::PathBuf::from(".");
        path.push("target");
        path.push("release");
        search_paths.push(path);
    }

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
                        load_plugin(&bin, context)?;
                    }
                }
            }
        }
    }

    Ok(())
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
            whole_stream_command(ToYAML),
            whole_stream_command(SortBy),
            whole_stream_command(Tags),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(FromCSV),
            whole_stream_command(FromTSV),
            whole_stream_command(FromINI),
            whole_stream_command(FromBSON),
            whole_stream_command(FromJSON),
            whole_stream_command(FromDB),
            whole_stream_command(FromSQLite),
            whole_stream_command(FromTOML),
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
            per_item_command(Cpy),
            whole_stream_command(Date),
            per_item_command(Mkdir),
            per_item_command(Move),
            whole_stream_command(Save),
            whole_stream_command(Table),
            whole_stream_command(VTable),
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
    let _ = rl.load_history("history.txt");

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

        let edit_mode = crate::data::config::config(Span::unknown())?
            .get("edit_mode")
            .map(|s| match s.as_string().unwrap().as_ref() {
                "vi" => EditMode::Vi,
                "emacs" => EditMode::Emacs,
                _ => EditMode::Emacs,
            })
            .unwrap_or(EditMode::Emacs);

        rl.set_edit_mode(edit_mode);

        let readline = rl.readline(&format!(
            "{}{}> ",
            cwd,
            match current_branch() {
                Some(s) => format!("({})", s),
                None => "".to_string(),
            }
        ));

        match process_line(readline, &mut context).await {
            LineResult::Success(line) => {
                rl.add_history_entry(line.clone());
            }

            LineResult::CtrlC => {
                if ctrlcbreak {
                    std::process::exit(0);
                } else {
                    context.with_host(|host| host.stdout("CTRL-C pressed (again to quit)"));
                    ctrlcbreak = true;
                    continue;
                }
            }

            LineResult::Error(mut line, err) => {
                rl.add_history_entry(line.clone());
                let diag = err.to_diagnostic();
                context.with_host(|host| {
                    let writer = host.err_termcolor();
                    line.push_str(" ");
                    let files = crate::parser::Files::new(line);
                    let _ = std::panic::catch_unwind(move || {
                        let _ = language_reporting::emit(
                            &mut writer.lock(),
                            &files,
                            &diag,
                            &language_reporting::DefaultConfig,
                        );
                    });
                })
            }

            LineResult::Break => {
                break;
            }
        }
        ctrlcbreak = false;
    }

    // we are ok if we can not save history
    let _ = rl.save_history("history.txt");

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
            let result = match crate::parser::parse(&line) {
                Err(err) => {
                    return LineResult::Error(line.clone(), err);
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let mut pipeline = match classify_pipeline(&result, ctx, &Text::from(line)) {
                Ok(pipeline) => pipeline,
                Err(err) => return LineResult::Error(line.clone(), err),
            };

            match pipeline.commands.last() {
                Some(ClassifiedCommand::External(_)) => {}
                _ => pipeline
                    .commands
                    .push(ClassifiedCommand::Internal(InternalCommand {
                        command: whole_stream_command(autoview::Autoview),
                        name_span: Span::unknown(),
                        args: hir::Call::new(
                            Box::new(hir::Expression::synthetic_string("autoview")),
                            None,
                            None,
                        ),
                    })),
            }

            let mut input = ClassifiedInputStream::new();

            let mut iter = pipeline.commands.into_iter().peekable();

            loop {
                let item: Option<ClassifiedCommand> = iter.next();
                let next: Option<&ClassifiedCommand> = iter.peek();

                input = match (item, next) {
                    (None, _) => break,

                    (Some(ClassifiedCommand::Expr(_)), _) => {
                        return LineResult::Error(
                            line.clone(),
                            ShellError::unimplemented("Expression-only commands"),
                        )
                    }

                    (_, Some(ClassifiedCommand::Expr(_))) => {
                        return LineResult::Error(
                            line.clone(),
                            ShellError::unimplemented("Expression-only commands"),
                        )
                    }

                    (
                        Some(ClassifiedCommand::Internal(left)),
                        Some(ClassifiedCommand::External(_)),
                    ) => match left.run(ctx, input, Text::from(line)).await {
                        Ok(val) => ClassifiedInputStream::from_input_stream(val),
                        Err(err) => return LineResult::Error(line.clone(), err),
                    },

                    (Some(ClassifiedCommand::Internal(left)), Some(_)) => {
                        match left.run(ctx, input, Text::from(line)).await {
                            Ok(val) => ClassifiedInputStream::from_input_stream(val),
                            Err(err) => return LineResult::Error(line.clone(), err),
                        }
                    }

                    (Some(ClassifiedCommand::Internal(left)), None) => {
                        match left.run(ctx, input, Text::from(line)).await {
                            Ok(val) => ClassifiedInputStream::from_input_stream(val),
                            Err(err) => return LineResult::Error(line.clone(), err),
                        }
                    }

                    (
                        Some(ClassifiedCommand::External(left)),
                        Some(ClassifiedCommand::External(_)),
                    ) => match left.run(ctx, input, StreamNext::External).await {
                        Ok(val) => val,
                        Err(err) => return LineResult::Error(line.clone(), err),
                    },

                    (Some(ClassifiedCommand::External(left)), Some(_)) => {
                        match left.run(ctx, input, StreamNext::Internal).await {
                            Ok(val) => val,
                            Err(err) => return LineResult::Error(line.clone(), err),
                        }
                    }

                    (Some(ClassifiedCommand::External(left)), None) => {
                        match left.run(ctx, input, StreamNext::Last).await {
                            Ok(val) => val,
                            Err(err) => return LineResult::Error(line.clone(), err),
                        }
                    }
                }
            }

            LineResult::Success(line.clone())
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
    command: &PipelineElement,
    context: &Context,
    source: &Text,
) -> Result<ClassifiedCommand, ShellError> {
    let call = command.call();

    match call {
        // If the command starts with `^`, treat it as an external command no matter what
        call if call.head().is_external() => {
            let name_span = call.head().expect_external();
            let name = name_span.slice(source);

            Ok(external_command(call, source, name.tagged(name_span)))
        }

        // Otherwise, if the command is a bare word, we'll need to triage it
        call if call.head().is_bare() => {
            let head = call.head();
            let name = head.source(source);

            match context.has_command(name) {
                // if the command is in the registry, it's an internal command
                true => {
                    let command = context.get_command(name);
                    let config = command.signature();

                    trace!(target: "nu::build_pipeline", "classifying {:?}", config);

                    let args: hir::Call = config.parse_args(call, &context, source)?;

                    trace!(target: "nu::build_pipeline", "args :: {}", args.debug(source));

                    Ok(ClassifiedCommand::Internal(InternalCommand {
                        command,
                        name_span: head.span().clone(),
                        args,
                    }))
                }

                // otherwise, it's an external command
                false => Ok(external_command(call, source, name.tagged(head.span()))),
            }
        }

        // If the command is something else (like a number or a variable), that is currently unsupported.
        // We might support `$somevar` as a curried command in the future.
        call => Err(ShellError::invalid_command(call.head().span())),
    }
}

// Classify this command as an external command, which doesn't give special meaning
// to nu syntactic constructs, and passes all arguments to the external command as
// strings.
fn external_command(
    call: &Tagged<CallNode>,
    source: &Text,
    name: Tagged<&str>,
) -> ClassifiedCommand {
    let arg_list_strings: Vec<Tagged<String>> = match call.children() {
        Some(args) => args
            .iter()
            .filter_map(|i| match i {
                TokenNode::Whitespace(_) => None,
                other => Some(Tagged::from_simple_spanned_item(
                    other.as_external_arg(source),
                    other.span(),
                )),
            })
            .collect(),
        None => vec![],
    };

    let (name, tag) = name.into_parts();

    ClassifiedCommand::External(ExternalCommand {
        name: name.to_string(),
        name_span: tag.span,
        args: arg_list_strings,
    })
}
