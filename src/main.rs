#[cfg(windows)]
use crossterm_winapi::{ConsoleMode, Handle};
use dialoguer::{
    console::{Style, Term},
    theme::ColorfulTheme,
    Select,
};
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_cli::{CliError, NuCompleter, NuHighlighter, NuValidator, NushellPrompt};
use nu_color_config::get_color_config;
use nu_command::create_default_context;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::{lex, parse, trim_quotes, Token, TokenContents};
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, ShellError, Span, Value, CONFIG_VARIABLE_ID,
};
use reedline::{Completer, CompletionActionHandler, DefaultHinter, LineBuffer, Prompt, Vi};
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[cfg(test)]
mod tests;

mod logger;

// Name of environment variable where the prompt could be stored
const PROMPT_COMMAND: &str = "PROMPT_COMMAND";
const PROMPT_INDICATOR: &str = "PROMPT_INDICATOR";
const PROMPT_INDICATOR_VI_INSERT: &str = "PROMPT_INDICATOR_VI_INSERT";
const PROMPT_INDICATOR_VI_VISUAL: &str = "PROMPT_INDICATOR_VI_VISUAL";
const PROMPT_MULTILINE_INDICATOR: &str = "PROMPT_MULTILINE_INDICATOR";

struct FuzzyCompletion {
    completer: Box<dyn Completer>,
}

impl CompletionActionHandler for FuzzyCompletion {
    fn handle(&mut self, present_buffer: &mut LineBuffer) {
        let completions = self
            .completer
            .complete(present_buffer.get_buffer(), present_buffer.offset());

        if completions.is_empty() {
            // do nothing
        } else if completions.len() == 1 {
            let span = completions[0].0;

            let mut offset = present_buffer.offset();
            offset += completions[0].1.len() - (span.end - span.start);

            // TODO improve the support for multiline replace
            present_buffer.replace(span.start..span.end, &completions[0].1);
            present_buffer.set_insertion_point(offset);
        } else {
            let selections: Vec<_> = completions.iter().map(|(_, string)| string).collect();

            let _ = crossterm::terminal::disable_raw_mode();
            println!();
            let theme = ColorfulTheme {
                active_item_style: Style::new().for_stderr().on_green().black(),
                ..Default::default()
            };
            let result = Select::with_theme(&theme)
                .default(0)
                .items(&selections[..])
                .interact_on_opt(&Term::stdout())
                .unwrap_or(None);
            let _ = crossterm::terminal::enable_raw_mode();

            if let Some(result) = result {
                let span = completions[result].0;

                let mut offset = present_buffer.offset();
                offset += completions[result].1.len() - (span.end - span.start);

                // TODO improve the support for multiline replace
                present_buffer.replace(span.start..span.end, &completions[result].1);
                present_buffer.set_insertion_point(offset);
            }
        }
    }
}

fn main() -> Result<()> {
    // miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    // Get initial current working directory.
    let init_cwd = get_init_cwd();
    let mut engine_state = create_default_context(&init_cwd);

    // TODO: make this conditional in the future
    // Ctrl-c protection section
    let ctrlc = Arc::new(AtomicBool::new(false));
    let handler_ctrlc = ctrlc.clone();
    let engine_state_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(engine_state_ctrlc);
    // End ctrl-c protection section

    if let Some(path) = std::env::args().nth(1) {
        let file = std::fs::read(&path).into_diagnostic()?;

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            trace!("parsing file: {}", path);

            let (output, err) = parse(&mut working_set, Some(&path), &file, false);
            if let Some(err) = err {
                report_error(&working_set, &err);

                std::process::exit(1);
            }
            (output, working_set.render())
        };

        if let Err(err) = engine_state.merge_delta(delta, None, &init_cwd) {
            let working_set = StateWorkingSet::new(&engine_state);
            report_error(&working_set, &err);
        }

        let mut stack = nu_protocol::engine::Stack::new();

        // First, set up env vars as strings only
        gather_parent_env_vars(&mut engine_state);

        // Set up our initial config to start from
        stack.vars.insert(
            CONFIG_VARIABLE_ID,
            Value::Record {
                cols: vec![],
                vals: vec![],
                span: Span { start: 0, end: 0 },
            },
        );

        let config = match stack.get_config() {
            Ok(config) => config,
            Err(e) => {
                let working_set = StateWorkingSet::new(&engine_state);

                report_error(&working_set, &e);
                Config::default()
            }
        };

        // Translate environment variables from Strings to Values
        if let Some(e) = convert_env_values(&mut engine_state, &stack, &config) {
            let working_set = StateWorkingSet::new(&engine_state);
            report_error(&working_set, &e);
            std::process::exit(1);
        }

        match eval_block(
            &engine_state,
            &mut stack,
            &block,
            PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
        ) {
            Ok(pipeline_data) => {
                for item in pipeline_data {
                    if let Value::Error { error } = item {
                        let working_set = StateWorkingSet::new(&engine_state);

                        report_error(&working_set, &error);

                        std::process::exit(1);
                    }
                    println!("{}", item.into_string("\n", &config));
                }

                // Next, let's check if there are any flags we want to pass to the main function
                let args: Vec<String> = std::env::args().skip(2).collect();

                if args.is_empty() && engine_state.find_decl(b"main").is_none() {
                    return Ok(());
                }

                let args = format!("main {}", args.join(" ")).as_bytes().to_vec();

                let (block, delta) = {
                    let mut working_set = StateWorkingSet::new(&engine_state);
                    let (output, err) = parse(&mut working_set, Some("<cmdline>"), &args, false);
                    if let Some(err) = err {
                        report_error(&working_set, &err);

                        std::process::exit(1);
                    }
                    (output, working_set.render())
                };

                let cwd = nu_engine::env::current_dir_str(&engine_state, &stack)?;

                if let Err(err) = engine_state.merge_delta(delta, Some(&mut stack), &cwd) {
                    let working_set = StateWorkingSet::new(&engine_state);
                    report_error(&working_set, &err);
                }

                match eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
                ) {
                    Ok(pipeline_data) => {
                        for item in pipeline_data {
                            if let Value::Error { error } = item {
                                let working_set = StateWorkingSet::new(&engine_state);

                                report_error(&working_set, &error);

                                std::process::exit(1);
                            }
                            println!("{}", item.into_string("\n", &config));
                        }
                    }
                    Err(err) => {
                        let working_set = StateWorkingSet::new(&engine_state);

                        report_error(&working_set, &err);

                        std::process::exit(1);
                    }
                }
            }
            Err(err) => {
                let working_set = StateWorkingSet::new(&engine_state);

                report_error(&working_set, &err);

                std::process::exit(1);
            }
        }

        Ok(())
    } else {
        use reedline::{FileBackedHistory, Reedline, Signal};

        let mut entry_num = 0;

        let mut nu_prompt = NushellPrompt::new();
        let mut stack = nu_protocol::engine::Stack::new();

        // First, set up env vars as strings only
        gather_parent_env_vars(&mut engine_state);

        // Set up our initial config to start from
        stack.vars.insert(
            CONFIG_VARIABLE_ID,
            Value::Record {
                cols: vec![],
                vals: vec![],
                span: Span::new(0, 0),
            },
        );

        #[cfg(feature = "plugin")]
        {
            // Reading signatures from signature file
            // The plugin.nu file stores the parsed signature collected from each registered plugin
            if let Some(mut plugin_path) = nu_path::config_dir() {
                // Path to store plugins signatures
                plugin_path.push("nushell");
                plugin_path.push("plugin.nu");
                engine_state.plugin_signatures = Some(plugin_path.clone());

                let plugin_filename = plugin_path.to_string_lossy().to_owned();

                if let Ok(contents) = std::fs::read_to_string(&plugin_path) {
                    eval_source(&mut engine_state, &mut stack, &contents, &plugin_filename);
                }
            }
        }

        // Load config startup file
        if let Some(mut config_path) = nu_path::config_dir() {
            config_path.push("nushell");

            // Create config directory if it does not exist
            if !config_path.exists() {
                if let Err(err) = std::fs::create_dir_all(&config_path) {
                    eprintln!("Failed to create config directory: {}", err);
                }
            } else {
                config_path.push("config.nu");

                if config_path.exists() {
                    // FIXME: remove this message when we're ready
                    println!("Loading config from: {:?}", config_path);
                    let config_filename = config_path.to_string_lossy().to_owned();

                    if let Ok(contents) = std::fs::read_to_string(&config_path) {
                        eval_source(&mut engine_state, &mut stack, &contents, &config_filename);
                    }
                }
            }
        }

        // Get the config
        let config = match stack.get_config() {
            Ok(config) => config,
            Err(e) => {
                let working_set = StateWorkingSet::new(&engine_state);

                report_error(&working_set, &e);
                Config::default()
            }
        };

        use logger::{configure, logger};

        logger(|builder| {
            configure(&config.log_level, builder)?;
            // trace_filters(self, builder)?;
            // debug_filters(self, builder)?;

            Ok(())
        })?;

        // Translate environment variables from Strings to Values
        if let Some(e) = convert_env_values(&mut engine_state, &stack, &config) {
            let working_set = StateWorkingSet::new(&engine_state);
            report_error(&working_set, &e);
        }

        let history_path = nu_path::config_dir().and_then(|mut history_path| {
            history_path.push("nushell");
            history_path.push("history.txt");

            if !history_path.exists() {
                // Creating an empty file to store the history
                match std::fs::File::create(&history_path) {
                    Ok(_) => Some(history_path),
                    Err(_) => None,
                }
            } else {
                Some(history_path)
            }
        });

        loop {
            let config = match stack.get_config() {
                Ok(config) => config,
                Err(e) => {
                    let working_set = StateWorkingSet::new(&engine_state);

                    report_error(&working_set, &e);
                    Config::default()
                }
            };

            //Reset the ctrl-c handler
            ctrlc.store(false, Ordering::SeqCst);

            let line_editor = Reedline::create()
                .into_diagnostic()?
                .with_completion_action_handler(Box::new(FuzzyCompletion {
                    completer: Box::new(NuCompleter::new(engine_state.clone())),
                }))
                .with_highlighter(Box::new(NuHighlighter {
                    engine_state: engine_state.clone(),
                    config: config.clone(),
                }))
                .with_animation(config.animate_prompt)
                // .with_completion_action_handler(Box::new(
                //     ListCompletionHandler::default().with_completer(Box::new(completer)),
                // ))
                .with_validator(Box::new(NuValidator {
                    engine_state: engine_state.clone(),
                }))
                .with_ansi_colors(config.use_ansi_coloring);
            //FIXME: if config.use_ansi_coloring is false then we should
            // turn off the hinter but I don't see any way to do that yet.

            let color_hm = get_color_config(&config);

            let line_editor = if let Some(history_path) = history_path.clone() {
                let history = std::fs::read_to_string(&history_path);
                if history.is_ok() {
                    line_editor
                        .with_hinter(Box::new(
                            DefaultHinter::default()
                                .with_history()
                                .with_style(color_hm["hints"]),
                        ))
                        .with_history(Box::new(
                            FileBackedHistory::with_file(
                                config.max_history_size as usize,
                                history_path.clone(),
                            )
                            .into_diagnostic()?,
                        ))
                        .into_diagnostic()?
                } else {
                    line_editor
                }
            } else {
                line_editor
            };

            // The line editor default mode is emacs mode. For the moment we only
            // need to check for vi mode
            let mut line_editor = if config.edit_mode == "vi" {
                let edit_mode = Box::new(Vi::default());
                line_editor.with_edit_mode(edit_mode)
            } else {
                line_editor
            };

            let prompt = update_prompt(&config, &engine_state, &stack, &mut nu_prompt);

            entry_num += 1;

            let input = line_editor.read_line(prompt);
            match input {
                Ok(Signal::Success(s)) => {
                    let tokens = lex(s.as_bytes(), 0, &[], &[], false);
                    // Check if this is a single call to a directory, if so auto-cd
                    let cwd = nu_engine::env::current_dir_str(&engine_state, &stack)?;
                    let path = nu_path::expand_path_with(&s, &cwd);
                    let orig = s.clone();

                    if (orig.starts_with('.')
                        || orig.starts_with('~')
                        || orig.starts_with('/')
                        || orig.starts_with('\\'))
                        && path.is_dir()
                        && tokens.0.len() == 1
                    {
                        // We have an auto-cd
                        //FIXME: this only changes the current scope, but instead this environment variable
                        //should probably be a block that loads the information from the state in the overlay
                        stack.add_env_var(
                            "PWD".into(),
                            Value::String {
                                val: s.clone(),
                                span: Span { start: 0, end: 0 },
                            },
                        );

                        continue;
                    }

                    trace!("eval source: {}", s);

                    eval_source(
                        &mut engine_state,
                        &mut stack,
                        &s,
                        &format!("entry #{}", entry_num),
                    );
                }
                Ok(Signal::CtrlC) => {
                    // `Reedline` clears the line content. New prompt is shown
                }
                Ok(Signal::CtrlD) => {
                    // When exiting clear to a new line
                    println!();
                    break;
                }
                Ok(Signal::CtrlL) => {
                    line_editor.clear_screen().into_diagnostic()?;
                }
                Err(err) => {
                    let message = err.to_string();
                    if !message.contains("duration") {
                        println!("Error: {:?}", err);
                    }
                }
            }
        }

        Ok(())
    }
}

// This fill collect environment variables from std::env and adds them to a stack.
//
// In order to ensure the values have spans, it first creates a dummy file, writes the collected
// env vars into it (in a "NAME"="value" format, quite similar to the output of the Unix 'env'
// tool), then uses the file to get the spans. The file stays in memory, no filesystem IO is done.
fn gather_parent_env_vars(engine_state: &mut EngineState) {
    // Some helper functions
    fn get_surround_char(s: &str) -> Option<char> {
        if s.contains('"') {
            if s.contains('\'') {
                None
            } else {
                Some('\'')
            }
        } else {
            Some('"')
        }
    }

    fn report_capture_error(engine_state: &EngineState, env_str: &str, msg: &str) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::LabeledError(
                format!("Environment variable was not captured: {}", env_str),
                msg.into(),
            ),
        );
    }

    fn put_env_to_fake_file(
        name: &str,
        val: &str,
        fake_env_file: &mut String,
        engine_state: &EngineState,
    ) {
        let (c_name, c_val) =
            if let (Some(cn), Some(cv)) = (get_surround_char(name), get_surround_char(val)) {
                (cn, cv)
            } else {
                // environment variable with its name or value containing both ' and " is ignored
                report_capture_error(
                    engine_state,
                    &format!("{}={}", name, val),
                    "Name or value should not contain both ' and \" at the same time.",
                );
                return;
            };

        fake_env_file.push(c_name);
        fake_env_file.push_str(name);
        fake_env_file.push(c_name);
        fake_env_file.push('=');
        fake_env_file.push(c_val);
        fake_env_file.push_str(val);
        fake_env_file.push(c_val);
        fake_env_file.push('\n');
    }

    let mut fake_env_file = String::new();

    // Make sure we always have PWD
    if std::env::var("PWD").is_err() {
        match std::env::current_dir() {
            Ok(cwd) => {
                put_env_to_fake_file(
                    "PWD",
                    &cwd.to_string_lossy(),
                    &mut fake_env_file,
                    engine_state,
                );
            }
            Err(e) => {
                // Could not capture current working directory
                let working_set = StateWorkingSet::new(engine_state);
                report_error(
                    &working_set,
                    &ShellError::LabeledError(
                        "Current directory not found".to_string(),
                        format!("Retrieving current directory failed: {:?}", e),
                    ),
                );
            }
        }
    }

    // Write all the env vars into a fake file
    for (name, val) in std::env::vars() {
        put_env_to_fake_file(&name, &val, &mut fake_env_file, engine_state);
    }

    // Lex the fake file, assign spans to all environment variables and add them
    // to stack
    let span_offset = engine_state.next_span_start();

    engine_state.add_file(
        "Host Environment Variables".to_string(),
        fake_env_file.as_bytes().to_vec(),
    );

    let (tokens, _) = lex(fake_env_file.as_bytes(), span_offset, &[], &[], true);

    for token in tokens {
        if let Token {
            contents: TokenContents::Item,
            span: full_span,
        } = token
        {
            let contents = engine_state.get_span_contents(&full_span);
            let (parts, _) = lex(contents, full_span.start, &[], &[b'='], true);

            let name = if let Some(Token {
                contents: TokenContents::Item,
                span,
            }) = parts.get(0)
            {
                let bytes = engine_state.get_span_contents(span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty name.",
                    );

                    continue;
                }

                let bytes = trim_quotes(bytes);
                String::from_utf8_lossy(bytes).to_string()
            } else {
                report_capture_error(
                    engine_state,
                    &String::from_utf8_lossy(contents),
                    "Got empty name.",
                );

                continue;
            };

            let value = if let Some(Token {
                contents: TokenContents::Item,
                span,
            }) = parts.get(2)
            {
                let bytes = engine_state.get_span_contents(span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty value.",
                    );

                    continue;
                }

                let bytes = trim_quotes(bytes);

                Value::String {
                    val: String::from_utf8_lossy(bytes).to_string(),
                    span: *span,
                }
            } else {
                report_capture_error(
                    engine_state,
                    &String::from_utf8_lossy(contents),
                    "Got empty value.",
                );

                continue;
            };

            // stack.add_env_var(name, value);
            engine_state.env_vars.insert(name, value);
        }
    }
}

fn print_pipeline_data(
    input: PipelineData,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    // If the table function is in the declarations, then we can use it
    // to create the table value that will be printed in the terminal

    let config = stack.get_config().unwrap_or_default();

    match input {
        PipelineData::StringStream(stream, _, _) => {
            for s in stream {
                print!("{}", s?);
                let _ = std::io::stdout().flush();
            }
            return Ok(());
        }
        PipelineData::ByteStream(stream, _, _) => {
            let mut address_offset = 0;
            for v in stream {
                let cfg = nu_pretty_hex::HexConfig {
                    title: false,
                    address_offset,
                    ..Default::default()
                };

                let v = v?;
                address_offset += v.len();

                let s = if v.iter().all(|x| x.is_ascii()) {
                    format!("{}", String::from_utf8_lossy(&v))
                } else {
                    nu_pretty_hex::config_hex(&v, cfg)
                };
                println!("{}", s);
            }
            return Ok(());
        }
        _ => {}
    }

    match engine_state.find_decl("table".as_bytes()) {
        Some(decl_id) => {
            let table =
                engine_state
                    .get_decl(decl_id)
                    .run(engine_state, stack, &Call::new(), input)?;

            for item in table {
                let stdout = std::io::stdout();

                if let Value::Error { error } = item {
                    return Err(error);
                }

                let mut out = item.into_string("\n", &config);
                out.push('\n');

                match stdout.lock().write_all(out.as_bytes()) {
                    Ok(_) => (),
                    Err(err) => eprintln!("{}", err),
                };
            }
        }
        None => {
            for item in input {
                let stdout = std::io::stdout();

                if let Value::Error { error } = item {
                    return Err(error);
                }

                let mut out = item.into_string("\n", &config);
                out.push('\n');

                match stdout.lock().write_all(out.as_bytes()) {
                    Ok(_) => (),
                    Err(err) => eprintln!("{}", err),
                };
            }
        }
    };

    Ok(())
}

fn get_prompt_indicators(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> (String, String, String, String) {
    let prompt_indicator = match stack.get_env_var(engine_state, PROMPT_INDICATOR) {
        Some(pi) => pi.into_string("", config),
        None => "〉".to_string(),
    };

    let prompt_vi_insert = match stack.get_env_var(engine_state, PROMPT_INDICATOR_VI_INSERT) {
        Some(pvii) => pvii.into_string("", config),
        None => ": ".to_string(),
    };

    let prompt_vi_visual = match stack.get_env_var(engine_state, PROMPT_INDICATOR_VI_VISUAL) {
        Some(pviv) => pviv.into_string("", config),
        None => "v ".to_string(),
    };

    let prompt_multiline = match stack.get_env_var(engine_state, PROMPT_MULTILINE_INDICATOR) {
        Some(pm) => pm.into_string("", config),
        None => "::: ".to_string(),
    };

    (
        prompt_indicator,
        prompt_vi_insert,
        prompt_vi_visual,
        prompt_multiline,
    )
}

fn update_prompt<'prompt>(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
    nu_prompt: &'prompt mut NushellPrompt,
) -> &'prompt dyn Prompt {
    // get the other indicators
    let (
        prompt_indicator_string,
        prompt_vi_insert_string,
        prompt_vi_visual_string,
        prompt_multiline_string,
    ) = get_prompt_indicators(config, engine_state, stack);

    let prompt_command_block_id = match stack.get_env_var(engine_state, PROMPT_COMMAND) {
        Some(v) => match v.as_block() {
            Ok(b) => b,
            Err(_) => {
                // apply the other indicators
                nu_prompt.update_all_prompt_strings(
                    None,
                    prompt_indicator_string,
                    prompt_vi_insert_string,
                    prompt_vi_visual_string,
                    prompt_multiline_string,
                );
                return nu_prompt as &dyn Prompt;
            }
        },
        None => {
            // apply the other indicators
            nu_prompt.update_all_prompt_strings(
                None,
                prompt_indicator_string,
                prompt_vi_insert_string,
                prompt_vi_visual_string,
                prompt_multiline_string,
            );
            return nu_prompt as &dyn Prompt;
        }
    };

    let block = engine_state.get_block(prompt_command_block_id);

    let mut stack = stack.clone();

    let evaluated_prompt = match eval_block(
        engine_state,
        &mut stack,
        block,
        PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
    ) {
        Ok(pipeline_data) => {
            // let config = stack.get_config().unwrap_or_default();
            pipeline_data.collect_string("", config)
        }
        Err(..) => {
            // If we can't run the custom prompt, give them the default
            // apply the other indicators
            nu_prompt.update_all_prompt_strings(
                None,
                prompt_indicator_string,
                prompt_vi_insert_string,
                prompt_vi_visual_string,
                prompt_multiline_string,
            );
            return nu_prompt as &dyn Prompt;
        }
    };

    match evaluated_prompt {
        Ok(evaluated_prompt) => {
            nu_prompt.update_all_prompt_strings(
                Some(evaluated_prompt),
                prompt_indicator_string,
                prompt_vi_insert_string,
                prompt_vi_visual_string,
                prompt_multiline_string,
            );
        }
        _ => nu_prompt.update_all_prompt_strings(
            None,
            prompt_indicator_string,
            prompt_vi_insert_string,
            prompt_vi_visual_string,
            prompt_multiline_string,
        ),
    }

    nu_prompt as &dyn Prompt
}

fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &str,
    fname: &str,
) -> bool {
    trace!("eval_source");

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source.as_bytes(),
            false,
        );
        if let Some(err) = err {
            report_error(&working_set, &err);
            return false;
        }

        (output, working_set.render())
    };

    let cwd = match nu_engine::env::current_dir_str(engine_state, stack) {
        Ok(p) => PathBuf::from(p),
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
            get_init_cwd()
        }
    };

    if let Err(err) = engine_state.merge_delta(delta, Some(stack), &cwd) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &err);
    }

    match eval_block(
        engine_state,
        stack,
        &block,
        PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
    ) {
        Ok(pipeline_data) => {
            if let Err(err) = print_pipeline_data(pipeline_data, engine_state, stack) {
                let working_set = StateWorkingSet::new(engine_state);

                report_error(&working_set, &err);

                return false;
            }

            // reset vt processing, aka ansi because illbehaved externals can break it
            #[cfg(windows)]
            {
                let _ = enable_vt_processing();
            }
        }
        Err(err) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);

            return false;
        }
    }

    true
}

#[cfg(windows)]
pub fn enable_vt_processing() -> Result<(), ShellError> {
    pub const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
    pub const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    // let mask = ENABLE_VIRTUAL_TERMINAL_PROCESSING;

    let console_mode = ConsoleMode::from(Handle::current_out_handle()?);
    let old_mode = console_mode.mode()?;

    // researching odd ansi behavior in windows terminal repo revealed that
    // enable_processed_output and enable_virtual_terminal_processing should be used
    // also, instead of checking old_mode & mask, just set the mode already

    // if old_mode & mask == 0 {
    console_mode
        .set_mode(old_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING)?;
    // }

    Ok(())
}

pub fn report_error(
    working_set: &StateWorkingSet,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) {
    eprintln!("Error: {:?}", CliError(error, working_set));
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = enable_vt_processing();
    }
}

fn get_init_cwd() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => match std::env::var("PWD".to_string()) {
            Ok(cwd) => PathBuf::from(cwd),
            Err(_) => match nu_path::home_dir() {
                Some(cwd) => cwd,
                None => PathBuf::new(),
            },
        },
    }
}
