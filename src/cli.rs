use crate::commands::autoview;
use crate::commands::classified::{
    ClassifiedCommand, ClassifiedInputStream, ClassifiedPipeline, ExternalCommand, InternalCommand,
    StreamNext,
};
use crate::commands::plugin::JsonRpc;
use crate::commands::plugin::{PluginCommand, PluginSink};
use crate::commands::static_command;
use crate::context::Context;
crate use crate::errors::ShellError;
use crate::git::current_branch;
use crate::object::Value;
use crate::parser::registry::Signature;
use crate::parser::{hir, Pipeline, PipelineElement, TokenNode};
use crate::prelude::*;

use log::{debug, trace};
use regex::Regex;
use rustyline::error::ReadlineError;
use rustyline::{self, ColorMode, Config, Editor};
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

impl<T> MaybeOwned<'a, T> {
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
    let request_raw = serde_json::to_string(&request).unwrap();
    stdin.write(format!("{}\n", request_raw).as_bytes())?;
    let path = dunce::canonicalize(path).unwrap();

    let mut input = String::new();
    match reader.read_line(&mut input) {
        Ok(_) => {
            let response = serde_json::from_str::<JsonRpc<Result<Signature, ShellError>>>(&input);
            match response {
                Ok(jrpc) => match jrpc.params {
                    Ok(params) => {
                        let fname = path.to_string_lossy();
                        if params.is_filter {
                            let fname = fname.to_string();
                            let name = params.name.clone();
                            context.add_commands(vec![static_command(PluginCommand::new(
                                name, fname, params,
                            ))]);
                            Ok(())
                        } else {
                            let fname = fname.to_string();
                            let name = params.name.clone();
                            context.add_commands(vec![static_command(PluginSink::new(
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
    }
}

fn load_plugins_in_dir(path: &std::path::PathBuf, context: &mut Context) -> Result<(), ShellError> {
    let re_bin = Regex::new(r"^nu_plugin_[A-Za-z_]+$").unwrap();
    let re_exe = Regex::new(r"^nu_plugin_[A-Za-z_]+\.exe$").unwrap();

    match std::fs::read_dir(path) {
        Ok(p) => {
            for entry in p {
                let entry = entry.unwrap();
                let filename = entry.file_name();
                let f_name = filename.to_string_lossy();
                if re_bin.is_match(&f_name) || re_exe.is_match(&f_name) {
                    let mut load_path = path.clone();
                    load_path.push(f_name.to_string());
                    load_plugin(&load_path, context)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn load_plugins(context: &mut Context) -> Result<(), ShellError> {
    match env::var_os("PATH") {
        Some(paths) => {
            for path in env::split_paths(&paths) {
                let _ = load_plugins_in_dir(&path, context);
            }
        }
        None => println!("PATH is not defined in the environment."),
    }

    // Also use our debug output for now
    let mut path = std::path::PathBuf::from(".");
    path.push("target");
    path.push("debug");

    let _ = load_plugins_in_dir(&path, context);

    // Also use our release output for now
    let mut path = std::path::PathBuf::from(".");
    path.push("target");
    path.push("release");

    let _ = load_plugins_in_dir(&path, context);

    Ok(())
}

pub async fn cli() -> Result<(), Box<dyn Error>> {
    let mut context = Context::basic()?;

    {
        use crate::commands::*;

        context.add_commands(vec![
            command("first", Box::new(first::first)),
            command("pick", Box::new(pick::pick)),
            command("from-array", Box::new(from_array::from_array)),
            command("from-ini", Box::new(from_ini::from_ini)),
            command("from-csv", Box::new(from_csv::from_csv)),
            command("from-json", Box::new(from_json::from_json)),
            command("from-toml", Box::new(from_toml::from_toml)),
            command("from-xml", Box::new(from_xml::from_xml)),
            command("ps", Box::new(ps::ps)),
            command("ls", Box::new(ls::ls)),
            command("cd", Box::new(cd::cd)),
            command("size", Box::new(size::size)),
            command("from-yaml", Box::new(from_yaml::from_yaml)),
            command("enter", Box::new(enter::enter)),
            command("n", Box::new(next::next)),
            command("p", Box::new(prev::prev)),
            command("debug", Box::new(debug::debug)),
            command("lines", Box::new(lines::lines)),
            command("pick", Box::new(pick::pick)),
            command("shells", Box::new(shells::shells)),
            command("split-column", Box::new(split_column::split_column)),
            command("split-row", Box::new(split_row::split_row)),
            command("lines", Box::new(lines::lines)),
            command("reject", Box::new(reject::reject)),
            command("trim", Box::new(trim::trim)),
            command("to-array", Box::new(to_array::to_array)),
            command("to-csv", Box::new(to_csv::to_csv)),
            command("to-json", Box::new(to_json::to_json)),
            command("to-toml", Box::new(to_toml::to_toml)),
            command("to-yaml", Box::new(to_yaml::to_yaml)),
            command("sort-by", Box::new(sort_by::sort_by)),
            command("tags", Box::new(tags::tags)),
            static_command(Get),
            //static_command(Cd),
            static_command(Remove),
            static_command(Open),
            static_command(Where),
            static_command(Config),
            static_command(SkipWhile),
            static_command(Exit),
            static_command(Clip),
            static_command(Autoview),
            static_command(Copycp),
            static_command(Date),
            static_command(Mkdir),
            static_command(Save),
            static_command(Table),
            static_command(VTable),
            static_command(Which),
        ]);
    }
    let _ = load_plugins(&mut context);

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    //let h = crate::shell::Helper::new(context.clone_commands());
    let mut rl: Editor<_> = Editor::with_config(config);

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    //rl.set_helper(Some(h));
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
                    context
                        .host
                        .lock()
                        .unwrap()
                        .stdout("CTRL-C pressed (again to quit)");
                    ctrlcbreak = true;
                    continue;
                }
            }

            LineResult::Error(mut line, err) => {
                rl.add_history_entry(line.clone());

                let diag = err.to_diagnostic();
                let host = context.host.lock().unwrap();
                let writer = host.err_termcolor();
                line.push_str(" ");
                let files = crate::parser::Files::new(line);

                language_reporting::emit(
                    &mut writer.lock(),
                    &files,
                    &diag,
                    &language_reporting::DefaultConfig,
                )
                .unwrap();
            }

            LineResult::Break => {
                break;
            }

            LineResult::FatalError(_, err) => {
                context
                    .host
                    .lock()
                    .unwrap()
                    .stdout(&format!("A surprising fatal error occurred.\n{:?}", err));
            }
        }
        ctrlcbreak = false;
    }
    rl.save_history("history.txt").unwrap();

    Ok(())
}

enum LineResult {
    Success(String),
    Error(String, ShellError),
    CtrlC,
    Break,

    #[allow(unused)]
    FatalError(String, ShellError),
}

impl std::ops::Try for LineResult {
    type Ok = Option<String>;
    type Error = (String, ShellError);

    fn into_result(self) -> Result<Option<String>, (String, ShellError)> {
        match self {
            LineResult::Success(s) => Ok(Some(s)),
            LineResult::Error(string, err) => Err((string, err)),
            LineResult::Break => Ok(None),
            LineResult::CtrlC => Ok(None),
            LineResult::FatalError(string, err) => Err((string, err)),
        }
    }
    fn from_error(v: (String, ShellError)) -> Self {
        LineResult::Error(v.0, v.1)
    }

    fn from_ok(v: Option<String>) -> Self {
        match v {
            None => LineResult::Break,
            Some(v) => LineResult::Success(v),
        }
    }
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

            let mut pipeline = classify_pipeline(&result, ctx, &Text::from(line))
                .map_err(|err| (line.clone(), err))?;

            match pipeline.commands.last() {
                Some(ClassifiedCommand::External(_)) => {}
                _ => pipeline
                    .commands
                    .push(ClassifiedCommand::Internal(InternalCommand {
                        command: static_command(autoview::Autoview),
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
        Err(ReadlineError::Eof) => {
            println!("CTRL-D");
            LineResult::Break
        }
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
        call if call.head().is_bare() => {
            let head = call.head();
            let name = head.source(source);

            match context.has_command(name) {
                true => {
                    let command = context.get_command(name);
                    let config = command.signature();

                    trace!(target: "nu::build_pipeline", "classifying {:?}", config);

                    let args: hir::Call = config.parse_args(call, context.registry(), source)?;

                    Ok(ClassifiedCommand::Internal(InternalCommand {
                        command,
                        name_span: head.span().clone(),
                        args,
                    }))
                }
                false => {
                    let arg_list_strings: Vec<Tagged<String>> = match call.children() {
                        //Some(args) => args.iter().map(|i| i.as_external_arg(source)).collect(),
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

                    Ok(ClassifiedCommand::External(ExternalCommand {
                        name: name.to_string(),
                        name_span: head.span().clone(),
                        args: arg_list_strings,
                    }))
                }
            }
        }

        call => Err(ShellError::diagnostic(
            language_reporting::Diagnostic::new(
                language_reporting::Severity::Error,
                "Invalid command",
            )
            .with_label(language_reporting::Label::new_primary(call.head().span())),
        )),
    }
}
