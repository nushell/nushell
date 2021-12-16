#[cfg(windows)]
use crossterm_winapi::{ConsoleMode, Handle};
use dialoguer::{
    console::{Style, Term},
    theme::ColorfulTheme,
    Select,
};
use miette::{IntoDiagnostic, Result};
use nu_cli::{CliError, NuCompleter, NuHighlighter, NuValidator, NushellPrompt};
use nu_command::create_default_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, ShellError, Span, Value, CONFIG_VARIABLE_ID,
};
use reedline::{Completer, CompletionActionHandler, DefaultPrompt, LineBuffer, Prompt};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[cfg(test)]
mod tests;

// Name of environment variable where the prompt could be stored
const PROMPT_COMMAND: &str = "PROMPT_COMMAND";

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

    let mut engine_state = create_default_context();

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
            let (output, err) = parse(&mut working_set, Some(&path), &file, false);
            if let Some(err) = err {
                report_error(&working_set, &err);

                std::process::exit(1);
            }
            (output, working_set.render())
        };

        if let Err(err) = engine_state.merge_delta(delta) {
            let working_set = StateWorkingSet::new(&engine_state);
            report_error(&working_set, &err);
        }

        let mut stack = nu_protocol::engine::Stack::new();

        for (k, v) in std::env::vars() {
            stack.add_env_var(k, v);
        }

        // Set up our initial config to start from
        stack.vars.insert(
            CONFIG_VARIABLE_ID,
            Value::Record {
                cols: vec![],
                vals: vec![],
                span: Span::unknown(),
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

        match eval_block(
            &engine_state,
            &mut stack,
            &block,
            PipelineData::new(Span::unknown()),
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

                if let Err(err) = engine_state.merge_delta(delta) {
                    let working_set = StateWorkingSet::new(&engine_state);
                    report_error(&working_set, &err);
                }

                match eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::new(Span::unknown()),
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

        let default_prompt = DefaultPrompt::new(1);
        let mut nu_prompt = NushellPrompt::new();
        let mut stack = nu_protocol::engine::Stack::new();

        for (k, v) in std::env::vars() {
            stack.add_env_var(k, v);
        }

        // Set up our initial config to start from
        stack.vars.insert(
            CONFIG_VARIABLE_ID,
            Value::Record {
                cols: vec![],
                vals: vec![],
                span: Span::unknown(),
            },
        );

        // Load config startup file
        if let Some(mut config_path) = nu_path::config_dir() {
            config_path.push("nushell");
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

        let history_path = if let Some(mut history_path) = nu_path::config_dir() {
            history_path.push("nushell");
            history_path.push("history.txt");

            Some(history_path)
        } else {
            None
        };

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

            let mut line_editor = if let Some(history_path) = history_path.clone() {
                line_editor
                    .with_history(Box::new(
                        FileBackedHistory::with_file(1000, history_path.clone())
                            .into_diagnostic()?,
                    ))
                    .into_diagnostic()?
            } else {
                line_editor
            };

            let prompt = update_prompt(
                PROMPT_COMMAND,
                &engine_state,
                &stack,
                &mut nu_prompt,
                &default_prompt,
            );

            entry_num += 1;

            let input = line_editor.read_line(prompt);
            match input {
                Ok(Signal::Success(s)) => {
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

fn print_pipeline_data(
    input: PipelineData,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    // If the table function is in the declarations, then we can use it
    // to create the table value that will be printed in the terminal

    let config = stack.get_config().unwrap_or_default();

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

fn update_prompt<'prompt>(
    env_variable: &str,
    engine_state: &EngineState,
    stack: &Stack,
    nu_prompt: &'prompt mut NushellPrompt,
    default_prompt: &'prompt DefaultPrompt,
) -> &'prompt dyn Prompt {
    let prompt_command = match stack.get_env_var(env_variable) {
        Some(prompt) => prompt,
        None => return default_prompt as &dyn Prompt,
    };

    // Checking if the PROMPT_COMMAND is the same to avoid evaluating constantly
    // the same command, thus saturating the contents in the EngineState
    if !nu_prompt.is_new_prompt(prompt_command.as_str()) {
        return nu_prompt as &dyn Prompt;
    }

    let block = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) = parse(&mut working_set, None, prompt_command.as_bytes(), false);
        if let Some(err) = err {
            report_error(&working_set, &err);
            return default_prompt as &dyn Prompt;
        }
        output
    };

    let mut stack = stack.clone();

    let evaluated_prompt = match eval_block(
        engine_state,
        &mut stack,
        &block,
        PipelineData::new(Span::unknown()),
    ) {
        Ok(pipeline_data) => {
            let config = stack.get_config().unwrap_or_default();
            pipeline_data.collect_string("", &config)
        }
        Err(..) => {
            // If we can't run the custom prompt, give them the default
            return default_prompt as &dyn Prompt;
        }
    };

    nu_prompt.update_prompt(prompt_command, evaluated_prompt);

    nu_prompt as &dyn Prompt
}

fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &str,
    fname: &str,
) -> bool {
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

    if let Err(err) = engine_state.merge_delta(delta) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &err);
    }

    match eval_block(
        engine_state,
        stack,
        &block,
        PipelineData::new(Span::unknown()),
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
