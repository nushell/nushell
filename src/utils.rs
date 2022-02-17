use log::trace;
use nu_cli::CliError;
use nu_engine::eval_block;
use nu_parser::{lex, parse, trim_quotes, Token, TokenContents};
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, ShellError, Span, Value,
};
use std::{io::Write, path::PathBuf};

// This will collect environment variables from std::env and adds them to a stack.
//
// In order to ensure the values have spans, it first creates a dummy file, writes the collected
// env vars into it (in a "NAME"="value" format, quite similar to the output of the Unix 'env'
// tool), then uses the file to get the spans. The file stays in memory, no filesystem IO is done.
pub(crate) fn gather_parent_env_vars(engine_state: &mut EngineState) {
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

    let mut stdout = std::io::stdout();

    if let PipelineData::RawStream(stream, _, _) = input {
        for s in stream {
            let _ = stdout.write(s?.as_binary()?);
        }
        return Ok(());
    }

    match engine_state.find_decl("table".as_bytes()) {
        Some(decl_id) => {
            let table = engine_state.get_decl(decl_id).run(
                engine_state,
                stack,
                &Call::new(Span::new(0, 0)),
                input,
            )?;

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

pub(crate) fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
) -> bool {
    trace!("eval_source");

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
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

    match eval_block(engine_state, stack, &block, input) {
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
    use crossterm_winapi::{ConsoleMode, Handle};

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

pub(crate) fn get_init_cwd() -> PathBuf {
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
