use nu_cmd_base::hook::eval_hook;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::{escape_quote_string, lex, parse, unescape_unquote_string, Token, TokenContents};
use nu_protocol::engine::debugger::{BasicDebugger, DebugContext, NoopDebugger, WithDebug, WithoutDebug};
use nu_protocol::engine::StateWorkingSet;
use nu_protocol::{
    engine::{EngineState, Stack},
    print_if_stream, PipelineData, ShellError, Span, Value,
};
use nu_protocol::{report_error, report_error_new};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use nu_utils::utils::perf;
use std::path::Path;
use std::sync::{Arc, Mutex};

// This will collect environment variables from std::env and adds them to a stack.
//
// In order to ensure the values have spans, it first creates a dummy file, writes the collected
// env vars into it (in a "NAME"="value" format, quite similar to the output of the Unix 'env'
// tool), then uses the file to get the spans. The file stays in memory, no filesystem IO is done.
//
// The "PWD" env value will be forced to `init_cwd`.
// The reason to use `init_cwd`:
//
// While gathering parent env vars, the parent `PWD` may not be the same as `current working directory`.
// Consider to the following command as the case (assume we execute command inside `/tmp`):
//
//     tmux split-window -v -c "#{pane_current_path}"
//
// Here nu execute external command `tmux`, and tmux starts a new `nushell`, with `init_cwd` value "#{pane_current_path}".
// But at the same time `PWD` still remains to be `/tmp`.
//
// In this scenario, the new `nushell`'s PWD should be "#{pane_current_path}" rather init_cwd.
pub fn gather_parent_env_vars(engine_state: &mut EngineState, init_cwd: &Path) {
    gather_env_vars(std::env::vars(), engine_state, init_cwd);
}

fn gather_env_vars(
    vars: impl Iterator<Item = (String, String)>,
    engine_state: &mut EngineState,
    init_cwd: &Path,
) {
    fn report_capture_error(engine_state: &EngineState, env_str: &str, msg: &str) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::GenericError {
                error: format!("Environment variable was not captured: {env_str}"),
                msg: "".into(),
                span: None,
                help: Some(msg.into()),
                inner: vec![],
            },
        );
    }

    fn put_env_to_fake_file(name: &str, val: &str, fake_env_file: &mut String) {
        fake_env_file.push_str(&escape_quote_string(name));
        fake_env_file.push('=');
        fake_env_file.push_str(&escape_quote_string(val));
        fake_env_file.push('\n');
    }

    let mut fake_env_file = String::new();
    // Write all the env vars into a fake file
    for (name, val) in vars {
        put_env_to_fake_file(&name, &val, &mut fake_env_file);
    }

    match init_cwd.to_str() {
        Some(cwd) => {
            put_env_to_fake_file("PWD", cwd, &mut fake_env_file);
        }
        None => {
            // Could not capture current working directory
            let working_set = StateWorkingSet::new(engine_state);
            report_error(
                &working_set,
                &ShellError::GenericError {
                    error: "Current directory is not a valid utf-8 path".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(format!(
                        "Retrieving current directory failed: {init_cwd:?} not a valid utf-8 path"
                    )),
                    inner: vec![],
                },
            );
        }
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
            let contents = engine_state.get_span_contents(full_span);
            let (parts, _) = lex(contents, full_span.start, &[], &[b'='], true);

            let name = if let Some(Token {
                contents: TokenContents::Item,
                span,
            }) = parts.first()
            {
                let mut working_set = StateWorkingSet::new(engine_state);
                let bytes = working_set.get_span_contents(*span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty name.",
                    );

                    continue;
                }

                let (bytes, err) = unescape_unquote_string(bytes, *span);
                if let Some(err) = err {
                    working_set.error(err);
                }

                if working_set.parse_errors.first().is_some() {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got unparsable name.",
                    );

                    continue;
                }

                bytes
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
                let mut working_set = StateWorkingSet::new(engine_state);
                let bytes = working_set.get_span_contents(*span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty value.",
                    );

                    continue;
                }

                let (bytes, err) = unescape_unquote_string(bytes, *span);
                if let Some(err) = err {
                    working_set.error(err);
                }

                if working_set.parse_errors.first().is_some() {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got unparsable value.",
                    );

                    continue;
                }

                Value::string(bytes, *span)
            } else {
                report_capture_error(
                    engine_state,
                    &String::from_utf8_lossy(contents),
                    "Got empty value.",
                );

                continue;
            };

            // stack.add_env_var(name, value);
            engine_state.add_env_var(name, value);
        }
    }
}

pub fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> bool {
    let start_time = std::time::Instant::now();

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
            false,
        );
        if let Some(warning) = working_set.parse_warnings.first() {
            report_error(&working_set, warning);
        }

        if let Some(err) = working_set.parse_errors.first() {
            set_last_exit_code(stack, 1);
            report_error(&working_set, err);
            return false;
        }

        (output, working_set.render())
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        set_last_exit_code(stack, 1);
        report_error_new(engine_state, &err);
        return false;
    }

    let debug_context = WithDebug;
    let debugger = Arc::new(Mutex::new(BasicDebugger::default()));
    println!("== start ==");
    println!("should debug: {}", debug_context.should_debug());

    let b = if allow_return {
        let res = eval_block_with_early_return(
            engine_state,
            stack,
            &block,
            input,
            false,
            false,
            debug_context,
            &Some(debugger.clone()),
        );
        res
    } else {
        eval_block(
            engine_state,
            stack,
            &block,
            input,
            false,
            false,
            // DEBUG TODO
            WithoutDebug,
            &None,
        )
    };

    match b {
        Ok(pipeline_data) => {
            let config = engine_state.get_config();
            let result;
            if let PipelineData::ExternalStream {
                stdout: stream,
                stderr: stderr_stream,
                exit_code,
                ..
            } = pipeline_data
            {
                result = print_if_stream(stream, stderr_stream, false, exit_code);
            } else if let Some(hook) = config.hooks.display_output.clone() {
                match eval_hook(
                    engine_state,
                    stack,
                    Some(pipeline_data),
                    vec![],
                    &hook,
                    "display_output",
                ) {
                    Err(err) => {
                        result = Err(err);
                    }
                    Ok(val) => {
                        result = val.print(engine_state, stack, false, false);
                    }
                }
            } else {
                result = pipeline_data.print(engine_state, stack, true, false);
            }

            debugger.lock().unwrap().report();
            println!("<<< stop >>>>");

            match result {
                Err(err) => {
                    let working_set = StateWorkingSet::new(engine_state);

                    report_error(&working_set, &err);

                    return false;
                }
                Ok(exit_code) => {
                    set_last_exit_code(stack, exit_code);
                }
            }

            // reset vt processing, aka ansi because illbehaved externals can break it
            #[cfg(windows)]
            {
                let _ = enable_vt_processing();
            }
        }
        Err(err) => {
            set_last_exit_code(stack, 1);

            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);

            return false;
        }
    }
    perf(
        &format!("eval_source {}", &fname),
        start_time,
        file!(),
        line!(),
        column!(),
        engine_state.get_config().use_ansi_coloring,
    );

    true
}

fn set_last_exit_code(stack: &mut Stack, exit_code: i64) {
    stack.add_env_var(
        "LAST_EXIT_CODE".to_string(),
        Value::int(exit_code, Span::unknown()),
    );
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gather_env_vars() {
        let mut engine_state = EngineState::new();
        let symbols = r##" !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~"##;

        gather_env_vars(
            [
                ("FOO".into(), "foo".into()),
                ("SYMBOLS".into(), symbols.into()),
                (symbols.into(), "symbols".into()),
            ]
            .into_iter(),
            &mut engine_state,
            Path::new("t"),
        );

        let env = engine_state.render_env_vars();

        assert!(
            matches!(env.get(&"FOO".to_string()), Some(&Value::String { val, .. }) if val == "foo")
        );
        assert!(
            matches!(env.get(&"SYMBOLS".to_string()), Some(&Value::String { val, .. }) if val == symbols)
        );
        assert!(
            matches!(env.get(&symbols.to_string()), Some(&Value::String { val, .. }) if val == "symbols")
        );
        assert!(env.get(&"PWD".to_string()).is_some());
        assert_eq!(env.len(), 4);
    }
}
