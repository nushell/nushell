use crate::commands::classified::block::run_block;
use crate::commands::classified::maybe_text_codec::{MaybeTextCodec, StringOrBinary};
use crate::context::Context;
use crate::git::current_branch;
use crate::path::canonicalize;
use crate::prelude::*;
use crate::shell::Helper;
use crate::EnvironmentSyncer;
use futures_codec::FramedRead;
use nu_errors::{ProximateShellError, ShellDiagnostic, ShellError};
use nu_protocol::hir::{ClassifiedCommand, Expression, InternalCommand, Literal, NamedArguments};
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};

use log::{debug, trace};
use rustyline::config::{ColorMode, CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::{self, config::Configurer, At, Cmd, Editor, KeyPress, Movement, Word};
use std::error::Error;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

pub fn register_plugins(context: &mut Context) -> Result<(), ShellError> {
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
            whole_stream_command(NuPlugin),
            // System/file operations
            whole_stream_command(Exec),
            whole_stream_command(Pwd),
            whole_stream_command(Ls),
            whole_stream_command(Du),
            whole_stream_command(Cd),
            whole_stream_command(Remove),
            whole_stream_command(Open),
            whole_stream_command(Config),
            whole_stream_command(ConfigGet),
            whole_stream_command(ConfigSet),
            whole_stream_command(ConfigSetInto),
            whole_stream_command(ConfigClear),
            whole_stream_command(ConfigLoad),
            whole_stream_command(ConfigRemove),
            whole_stream_command(ConfigPath),
            whole_stream_command(Help),
            whole_stream_command(History),
            whole_stream_command(Save),
            whole_stream_command(Touch),
            whole_stream_command(Cpy),
            whole_stream_command(Date),
            whole_stream_command(DateNow),
            whole_stream_command(DateUTC),
            whole_stream_command(DateFormat),
            whole_stream_command(Cal),
            whole_stream_command(Mkdir),
            whole_stream_command(Mv),
            whole_stream_command(Kill),
            whole_stream_command(Version),
            whole_stream_command(Clear),
            whole_stream_command(What),
            whole_stream_command(Which),
            whole_stream_command(Debug),
            whole_stream_command(Alias),
            whole_stream_command(WithEnv),
            whole_stream_command(Do),
            whole_stream_command(Sleep),
            // Statistics
            whole_stream_command(Size),
            whole_stream_command(Count),
            whole_stream_command(Benchmark),
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
            whole_stream_command(Split),
            whole_stream_command(SplitColumn),
            whole_stream_command(SplitRow),
            whole_stream_command(SplitChars),
            whole_stream_command(Lines),
            whole_stream_command(Trim),
            whole_stream_command(Echo),
            whole_stream_command(Parse),
            whole_stream_command(Str),
            whole_stream_command(StrToDecimal),
            whole_stream_command(StrToInteger),
            whole_stream_command(StrDowncase),
            whole_stream_command(StrUpcase),
            whole_stream_command(StrCapitalize),
            whole_stream_command(StrFindReplace),
            whole_stream_command(StrFrom),
            whole_stream_command(StrSubstring),
            whole_stream_command(StrSet),
            whole_stream_command(StrToDatetime),
            whole_stream_command(StrContains),
            whole_stream_command(StrIndexOf),
            whole_stream_command(StrTrim),
            whole_stream_command(StrTrimLeft),
            whole_stream_command(StrTrimRight),
            whole_stream_command(StrStartsWith),
            whole_stream_command(StrEndsWith),
            whole_stream_command(StrCollect),
            whole_stream_command(StrLength),
            whole_stream_command(StrReverse),
            whole_stream_command(StrCamelCase),
            whole_stream_command(StrPascalCase),
            whole_stream_command(StrKebabCase),
            whole_stream_command(StrSnakeCase),
            whole_stream_command(StrScreamingSnakeCase),
            whole_stream_command(BuildString),
            whole_stream_command(Ansi),
            whole_stream_command(Char),
            // Column manipulation
            whole_stream_command(MoveColumn),
            whole_stream_command(Reject),
            whole_stream_command(Select),
            whole_stream_command(Get),
            whole_stream_command(Update),
            whole_stream_command(Insert),
            whole_stream_command(IntoInt),
            whole_stream_command(SplitBy),
            // Row manipulation
            whole_stream_command(Reverse),
            whole_stream_command(Append),
            whole_stream_command(Prepend),
            whole_stream_command(SortBy),
            whole_stream_command(GroupBy),
            whole_stream_command(GroupByDate),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(Every),
            whole_stream_command(Nth),
            whole_stream_command(Drop),
            whole_stream_command(Format),
            whole_stream_command(Where),
            whole_stream_command(If),
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
            whole_stream_command(EachGroup),
            whole_stream_command(EachWindow),
            whole_stream_command(IsEmpty),
            // Table manipulation
            whole_stream_command(Move),
            whole_stream_command(Merge),
            whole_stream_command(Shuffle),
            whole_stream_command(Wrap),
            whole_stream_command(Pivot),
            whole_stream_command(Headers),
            whole_stream_command(Reduce),
            // Data processing
            whole_stream_command(Histogram),
            whole_stream_command(Autoenv),
            whole_stream_command(AutoenvTrust),
            whole_stream_command(AutoenvUnTrust),
            whole_stream_command(Math),
            whole_stream_command(MathAverage),
            whole_stream_command(MathEval),
            whole_stream_command(MathMedian),
            whole_stream_command(MathMinimum),
            whole_stream_command(MathMode),
            whole_stream_command(MathMaximum),
            whole_stream_command(MathStddev),
            whole_stream_command(MathSummation),
            whole_stream_command(MathVariance),
            whole_stream_command(MathProduct),
            // File format output
            whole_stream_command(To),
            whole_stream_command(ToCSV),
            whole_stream_command(ToHTML),
            whole_stream_command(ToJSON),
            whole_stream_command(ToMarkdown),
            whole_stream_command(ToTOML),
            whole_stream_command(ToTSV),
            whole_stream_command(ToURL),
            whole_stream_command(ToYAML),
            whole_stream_command(ToXML),
            // File format input
            whole_stream_command(From),
            whole_stream_command(FromCSV),
            whole_stream_command(FromEML),
            whole_stream_command(FromTSV),
            whole_stream_command(FromSSV),
            whole_stream_command(FromINI),
            whole_stream_command(FromJSON),
            whole_stream_command(FromODS),
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
            // Random value generation
            whole_stream_command(Random),
            whole_stream_command(RandomBool),
            whole_stream_command(RandomDice),
            #[cfg(feature = "uuid_crate")]
            whole_stream_command(RandomUUID),
            whole_stream_command(RandomInteger),
            // Path
            whole_stream_command(PathBasename),
            whole_stream_command(PathCommand),
            whole_stream_command(PathDirname),
            whole_stream_command(PathExists),
            whole_stream_command(PathExpand),
            whole_stream_command(PathExtension),
            whole_stream_command(PathFilestem),
            whole_stream_command(PathType),
            // Url
            whole_stream_command(UrlCommand),
            whole_stream_command(UrlScheme),
            whole_stream_command(UrlPath),
            whole_stream_command(UrlHost),
            whole_stream_command(UrlQuery),
        ]);

        #[cfg(feature = "clipboard-cli")]
        {
            context.add_commands(vec![whole_stream_command(crate::commands::clip::Clip)]);
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

    let _ = register_plugins(&mut context);

    #[cfg(feature = "ctrlc")]
    {
        let cc = context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        if context.ctrl_c.load(Ordering::SeqCst) {
            context.ctrl_c.store(false, Ordering::SeqCst);
        }
    }

    // before we start up, let's run our startup commands
    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
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

pub fn create_rustyline_configuration() -> (Editor<Helper>, IndexMap<String, Value>) {
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

    let config = match config::config(Tag::unknown()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Config could not be loaded.");
            if let ShellError {
                error: ProximateShellError::Diagnostic(ShellDiagnostic { diagnostic }),
                ..
            } = e
            {
                eprintln!("{}", diagnostic.message);
            }
            IndexMap::new()
        }
    };

    if let Ok(config) = config::config(Tag::unknown()) {
        if let Some(line_editor_vars) = config.get("line_editor") {
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
                            Ok(s) if s.to_lowercase() == "emacs" => {
                                rustyline::config::EditMode::Emacs
                            }
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
                            Ok(s) if s.to_lowercase() == "none" => {
                                rustyline::config::BellStyle::None
                            }
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
                            Ok(s) if s.to_lowercase() == "disabled" => {
                                rustyline::ColorMode::Disabled
                            }
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
    }

    (rl, config)
}

/// The entry point for the CLI. Will register all known internal commands, load experimental commands, load plugins, then prepare the prompt and line reader for input.
pub async fn cli(
    mut syncer: EnvironmentSyncer,
    mut context: Context,
) -> Result<(), Box<dyn Error>> {
    let configuration = nu_data::config::NuConfig::new();
    let history_path = crate::commands::history::history_path(&configuration);

    let (mut rl, config) = create_rustyline_configuration();

    // we are ok if history does not exist
    let _ = rl.load_history(&history_path);

    let skip_welcome_message = config
        .get("skip_welcome_message")
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

    #[cfg(feature = "ctrlc")]
    {
        let cc = context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }
    let mut ctrlcbreak = false;

    // before we start up, let's run our startup commands
    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
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

        let hinter = init_hinter(&config);

        rl.set_helper(Some(crate::shell::Helper::new(context.clone(), hinter)));

        let colored_prompt = {
            if let Some(prompt) = config.get("prompt") {
                let prompt_line = prompt.as_string()?;

                match nu_parser::lite_parse(&prompt_line, 0).map_err(ShellError::from) {
                    Ok(result) => {
                        let mut prompt_block =
                            nu_parser::classify_block(&result, context.registry());

                        let env = context.get_env();

                        prompt_block.block.expand_it_usage();

                        match run_block(
                            &prompt_block.block,
                            &mut context,
                            InputStream::empty(),
                            &Value::nothing(),
                            &IndexMap::new(),
                            &env,
                        )
                        .await
                        {
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
                    Err(e) => {
                        crate::cli::print_err(e, &Text::from(prompt_line));
                        context.clear_errors();

                        "> ".to_string()
                    }
                }
            } else {
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
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);
                context.maybe_print_errors(Text::from(line));
            }

            LineResult::Error(line, err) => {
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);

                context.with_host(|_host| {
                    print_err(err, &Text::from(line.clone()));
                });

                context.maybe_print_errors(Text::from(line.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| s.value.expect_string() == "true")
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

fn init_hinter(config: &IndexMap<String, Value>) -> Option<rustyline::hint::HistoryHinter> {
    // Show hints unless explicitly disabled in config
    if let Some(line_editor_vars) = config.get("line_editor") {
        for (idx, value) in line_editor_vars.row_entries() {
            if idx == "show_hints" && value.expect_string() == "false" {
                return None;
            }
        }
    }
    Some(rustyline::hint::HistoryHinter {})
}

fn chomp_newline(s: &str) -> &str {
    if s.ends_with('\n') {
        &s[..s.len() - 1]
    } else {
        s
    }
}

#[derive(Debug)]
pub enum LineResult {
    Success(String),
    Error(String, ShellError),
    CtrlC,
    Break,
}

pub async fn parse_and_eval(line: &str, ctx: &mut Context) -> Result<String, ShellError> {
    let line = if line.ends_with('\n') {
        &line[..line.len() - 1]
    } else {
        line
    };

    let lite_result = nu_parser::lite_parse(&line, 0)?;

    // TODO ensure the command whose examples we're testing is actually in the pipeline
    let mut classified_block = nu_parser::classify_block(&lite_result, ctx.registry());
    classified_block.block.expand_it_usage();

    let input_stream = InputStream::empty();
    let env = ctx.get_env();

    run_block(
        &classified_block.block,
        ctx,
        input_stream,
        &Value::nothing(),
        &IndexMap::new(),
        &env,
    )
    .await?
    .collect_string(Tag::unknown())
    .await
    .map(|x| x.item)
}

/// Process the line by parsing the text to turn it into commands, classify those commands so that we understand what is being called in the pipeline, and then run this pipeline
pub async fn process_line(
    readline: Result<String, ReadlineError>,
    ctx: &mut Context,
    redirect_stdin: bool,
    cli_mode: bool,
) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);
            ctx.raw_input = line.to_string();

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

            classified_block.block.expand_it_usage();

            trace!("{:#?}", classified_block);
            let env = ctx.get_env();
            match run_block(
                &classified_block.block,
                ctx,
                input_stream,
                &Value::nothing(),
                &IndexMap::new(),
                &env,
            )
            .await
            {
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
                        registry: ctx.registry.clone(),
                        name: Tag::unknown(),
                        raw_input: line.to_string(),
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
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::Break,
        Err(err) => {
            outln!("Error: {:?}", err);
            LineResult::Break
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
        if let Ok(lite_block) = nu_parser::lite_parse(&data, 0) {
            let context = crate::context::Context::basic().unwrap();
            let _ = nu_parser::classify_block(&lite_block, context.registry());
        }
        true
    }
}
