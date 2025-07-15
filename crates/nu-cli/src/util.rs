#![allow(clippy::byte_char_slices)]

use nu_cmd_base::hook::eval_hook;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::{Token, TokenContents, lex, parse, unescape_unquote_string};
use nu_protocol::{
    PipelineData, ShellError, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error::report_compile_error,
    report_parse_error, report_parse_warning, report_shell_error,
};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use nu_utils::{escape_quote_string, perf};
use std::path::Path;

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
        report_shell_error(
            engine_state,
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
            report_shell_error(
                engine_state,
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
        "Host Environment Variables".into(),
        fake_env_file.as_bytes().into(),
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

                if !working_set.parse_errors.is_empty() {
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

                if !working_set.parse_errors.is_empty() {
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

/// Print a pipeline with formatting applied based on display_output hook.
///
/// This function should be preferred when printing values resulting from a completed evaluation.
/// For values printed as part of a command's execution, such as values printed by the `print` command,
/// the `PipelineData::print_table` function should be preferred instead as it is not config-dependent.
///
/// `no_newline` controls if we need to attach newline character to output.
pub fn print_pipeline(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    pipeline: PipelineData,
    no_newline: bool,
) -> Result<(), ShellError> {
    if let Some(hook) = engine_state.get_config().hooks.display_output.clone() {
        let pipeline = eval_hook(
            engine_state,
            stack,
            Some(pipeline),
            vec![],
            &hook,
            "display_output",
        )?;
        pipeline.print_raw(engine_state, no_newline, false)
    } else {
        // if display_output isn't set, we should still prefer to print with some formatting
        pipeline.print_table(engine_state, stack, no_newline, false)
    }
}

pub fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> i32 {
    let start_time = std::time::Instant::now();

    let exit_code = match evaluate_source(engine_state, stack, source, fname, input, allow_return) {
        Ok(failed) => {
            let code = failed.into();
            stack.set_last_exit_code(code, Span::unknown());
            code
        }
        Err(err) => {
            report_shell_error(engine_state, &err);
            let code = err.exit_code();
            stack.set_last_error(&err);
            code.unwrap_or(0)
        }
    };

    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = enable_vt_processing();
    }

    perf!(
        &format!("eval_source {}", &fname),
        start_time,
        engine_state
            .get_config()
            .use_ansi_coloring
            .get(engine_state)
    );

    exit_code
}

fn evaluate_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> Result<bool, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
            false,
        );
        if let Some(warning) = working_set.parse_warnings.first() {
            report_parse_warning(&working_set, warning);
        }

        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(&working_set, err);
            return Ok(true);
        }

        if let Some(err) = working_set.compile_errors.first() {
            report_compile_error(&working_set, err);
            return Ok(true);
        }

        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let pipeline = if allow_return {
        eval_block_with_early_return::<WithoutDebug>(engine_state, stack, &block, input)
    } else {
        eval_block::<WithoutDebug>(engine_state, stack, &block, input)
    }?;

    let no_newline = matches!(&pipeline, &PipelineData::ByteStream(..));
    print_pipeline(engine_state, stack, pipeline, no_newline)?;

    Ok(false)
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

        assert!(matches!(env.get("FOO"), Some(&Value::String { val, .. }) if val == "foo"));
        assert!(matches!(env.get("SYMBOLS"), Some(&Value::String { val, .. }) if val == symbols));
        assert!(matches!(env.get(symbols), Some(&Value::String { val, .. }) if val == "symbols"));
        assert!(env.contains_key("PWD"));
        assert_eq!(env.len(), 4);
    }
}
