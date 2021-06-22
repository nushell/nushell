use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, BufCodecReader};
use nu_engine::{MaybeTextCodec, StringOrBinary};
use nu_test_support::NATIVE_PATH_ENV_VAR;
use parking_lot::Mutex;

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::{borrow::Cow, io::BufReader};

use log::trace;

use nu_errors::ShellError;
use nu_protocol::hir::Expression;
use nu_protocol::hir::{ExternalCommand, ExternalRedirection};
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
use nu_source::Tag;

pub(crate) fn run_external_command(
    command: ExternalCommand,
    context: &mut EvaluationContext,
    input: InputStream,
    external_redirection: ExternalRedirection,
) -> Result<InputStream, ShellError> {
    trace!(target: "nu::run::external", "-> {}", command.name);

    context.sync_path_to_env();
    if !context.host().lock().is_external_cmd(&command.name) {
        return Err(ShellError::labeled_error(
            "Command not found",
            format!("command {} not found", &command.name),
            &command.name_tag,
        ));
    }

    run_with_stdin(command, context, input, external_redirection)
}

#[allow(unused)]
fn trim_double_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('"'), Some('"')) => chars.collect(),
        _ => input.to_string(),
    }
}

#[allow(unused)]
fn escape_where_needed(input: &str) -> String {
    input.split(' ').join("\\ ").split('\'').join("\\'")
}

fn run_with_stdin(
    command: ExternalCommand,
    context: &mut EvaluationContext,
    input: InputStream,
    external_redirection: ExternalRedirection,
) -> Result<InputStream, ShellError> {
    let path = context.shell_manager().path();

    let mut command_args = vec![];
    for arg in command.args.iter() {
        let is_literal = matches!(arg.expr, Expression::Literal(_));
        let value = evaluate_baseline_expr(arg, context)?;

        // Skip any arguments that don't really exist, treating them as optional
        // FIXME: we may want to preserve the gap in the future, though it's hard to say
        // what value we would put in its place.
        if value.value.is_none() {
            continue;
        }

        // Do the cleanup that we need to do on any argument going out:
        match &value.value {
            UntaggedValue::Table(table) => {
                for t in table {
                    match &t.value {
                        UntaggedValue::Primitive(_) => {
                            command_args.push((
                                t.convert_to_string().trim_end_matches('\n').to_string(),
                                is_literal,
                            ));
                        }
                        _ => {
                            return Err(ShellError::labeled_error(
                                "Could not convert to positional arguments",
                                "could not convert to positional arguments",
                                value.tag(),
                            ));
                        }
                    }
                }
            }
            _ => {
                let trimmed_value_string = value.as_string()?.trim_end_matches('\n').to_string();
                //let trimmed_value_string = trim_quotes(&trimmed_value_string);
                command_args.push((trimmed_value_string, is_literal));
            }
        }
    }

    let process_args = command_args
        .iter()
        .map(|(arg, _is_literal)| {
            let arg = nu_path::expand_tilde_string(Cow::Borrowed(arg));

            #[cfg(not(windows))]
            {
                if !_is_literal {
                    let escaped = escape_double_quotes(&arg);
                    add_double_quotes(&escaped)
                } else {
                    let trimmed = trim_double_quotes(&arg);
                    if trimmed != arg {
                        escape_where_needed(&trimmed)
                    } else {
                        trimmed
                    }
                }
            }
            #[cfg(windows)]
            {
                if let Some(unquoted) = remove_quotes(&arg) {
                    unquoted.to_string()
                } else {
                    arg.to_string()
                }
            }
        })
        .collect::<Vec<String>>();

    spawn(
        &command,
        &path,
        &process_args[..],
        input,
        external_redirection,
        &context.scope,
    )
}

fn spawn(
    command: &ExternalCommand,
    path: &str,
    args: &[String],
    input: InputStream,
    external_redirection: ExternalRedirection,
    scope: &Scope,
) -> Result<InputStream, ShellError> {
    let command = command.clone();

    let mut process = {
        #[cfg(windows)]
        {
            let mut process = Command::new("cmd");
            process.arg("/c");
            process.arg(&command.name);
            for arg in args {
                // Clean the args before we use them:
                // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
                // cmd.exe needs to have a caret to escape a pipe
                let arg = arg.replace("|", "^|");
                process.arg(&arg);
            }
            process
        }

        #[cfg(not(windows))]
        {
            let cmd_with_args = vec![command.name.clone(), args.join(" ")].join(" ");
            let mut process = Command::new("sh");
            process.arg("-c").arg(cmd_with_args);
            process
        }
    };

    process.current_dir(path);
    trace!(target: "nu::run::external", "cwd = {:?}", &path);

    process.env_clear();
    process.envs(scope.get_env_vars());

    // We want stdout regardless of what
    // we are doing ($it case or pipe stdin)
    match external_redirection {
        ExternalRedirection::Stdout => {
            process.stdout(Stdio::piped());
            trace!(target: "nu::run::external", "set up stdout pipe");
        }
        ExternalRedirection::Stderr => {
            process.stderr(Stdio::piped());
            trace!(target: "nu::run::external", "set up stderr pipe");
        }
        ExternalRedirection::StdoutAndStderr => {
            process.stdout(Stdio::piped());
            trace!(target: "nu::run::external", "set up stdout pipe");
            process.stderr(Stdio::piped());
            trace!(target: "nu::run::external", "set up stderr pipe");
        }
        _ => {}
    }

    // open since we have some contents for stdin
    if !input.is_empty() {
        process.stdin(Stdio::piped());
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built command {:?}", process);

    // TODO Switch to async_std::process once it's stabilized
    match process.spawn() {
        Ok(mut child) => {
            let (tx, rx) = mpsc::sync_channel(0);

            let mut stdin = child.stdin.take();

            let stdin_write_tx = tx.clone();
            let stdout_read_tx = tx;
            let stdin_name_tag = command.name_tag.clone();
            let stdout_name_tag = command.name_tag;

            std::thread::spawn(move || {
                if !input.is_empty() {
                    let mut stdin_write = stdin
                        .take()
                        .expect("Internal error: could not get stdin pipe for external command");

                    for value in input {
                        match &value.value {
                            UntaggedValue::Primitive(Primitive::Nothing) => continue,
                            UntaggedValue::Primitive(Primitive::String(s)) => {
                                if stdin_write.write(s.as_bytes()).is_err() {
                                    // Other side has closed, so exit
                                    return Ok(());
                                }
                            }
                            UntaggedValue::Primitive(Primitive::Binary(b)) => {
                                if stdin_write.write(b).is_err() {
                                    // Other side has closed, so exit
                                    return Ok(());
                                }
                            }
                            unsupported => {
                                println!("Unsupported: {:?}", unsupported);
                                let _ = stdin_write_tx.send(Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        format!(
                                            "Received unexpected type from pipeline ({})",
                                            unsupported.type_name()
                                        ),
                                        format!(
                                            "expected a string, got {} as input",
                                            unsupported.type_name()
                                        ),
                                        stdin_name_tag.clone(),
                                    )),
                                    tag: stdin_name_tag,
                                }));
                                return Err(());
                            }
                        };
                    }
                }

                Ok(())
            });

            std::thread::spawn(move || {
                if external_redirection == ExternalRedirection::Stdout
                    || external_redirection == ExternalRedirection::StdoutAndStderr
                {
                    let stdout = if let Some(stdout) = child.stdout.take() {
                        stdout
                    } else {
                        let _ = stdout_read_tx.send(Ok(Value {
                            value: UntaggedValue::Error(ShellError::labeled_error(
                                "Can't redirect the stdout for external command",
                                "can't redirect stdout",
                                &stdout_name_tag,
                            )),
                            tag: stdout_name_tag,
                        }));
                        return Err(());
                    };

                    // let file = futures::io::AllowStdIo::new(stdout);
                    // let stream = FramedRead::new(file, MaybeTextCodec::default());
                    let buf_read = BufReader::new(stdout);
                    let buf_codec = BufCodecReader::new(buf_read, MaybeTextCodec::default());

                    for line in buf_codec {
                        match line {
                            Ok(line) => match line {
                                StringOrBinary::String(s) => {
                                    let result = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Primitive(Primitive::String(
                                            s.clone(),
                                        )),
                                        tag: stdout_name_tag.clone(),
                                    }));

                                    if result.is_err() {
                                        break;
                                    }
                                }
                                StringOrBinary::Binary(b) => {
                                    let result = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Primitive(Primitive::Binary(
                                            b.into_iter().collect(),
                                        )),
                                        tag: stdout_name_tag.clone(),
                                    }));

                                    if result.is_err() {
                                        break;
                                    }
                                }
                            },
                            Err(e) => {
                                // If there's an exit status, it makes sense that we may error when
                                // trying to read from its stdout pipe (likely been closed). In that
                                // case, don't emit an error.
                                let should_error = match child.wait() {
                                    Ok(exit_status) => !exit_status.success(),
                                    Err(_) => true,
                                };

                                if should_error {
                                    let _ = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Error(ShellError::labeled_error(
                                            format!("Unable to read from stdout ({})", e),
                                            "unable to read from stdout",
                                            &stdout_name_tag,
                                        )),
                                        tag: stdout_name_tag.clone(),
                                    }));
                                }

                                return Ok(());
                            }
                        }
                    }
                }
                if external_redirection == ExternalRedirection::Stderr
                    || external_redirection == ExternalRedirection::StdoutAndStderr
                {
                    let stderr = if let Some(stderr) = child.stderr.take() {
                        stderr
                    } else {
                        let _ = stdout_read_tx.send(Ok(Value {
                            value: UntaggedValue::Error(ShellError::labeled_error(
                                "Can't redirect the stderr for external command",
                                "can't redirect stderr",
                                &stdout_name_tag,
                            )),
                            tag: stdout_name_tag,
                        }));
                        return Err(());
                    };

                    // let file = futures::io::AllowStdIo::new(stderr);
                    // let stream = FramedRead::new(file, MaybeTextCodec::default());
                    let buf_reader = BufReader::new(stderr);
                    let buf_codec = BufCodecReader::new(buf_reader, MaybeTextCodec::default());

                    for line in buf_codec {
                        match line {
                            Ok(line) => match line {
                                StringOrBinary::String(s) => {
                                    let result = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Error(
                                            ShellError::untagged_runtime_error(s),
                                        ),
                                        tag: stdout_name_tag.clone(),
                                    }));

                                    if result.is_err() {
                                        break;
                                    }
                                }
                                StringOrBinary::Binary(_) => {
                                    let result = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Error(
                                            ShellError::untagged_runtime_error("<binary stderr>"),
                                        ),
                                        tag: stdout_name_tag.clone(),
                                    }));

                                    if result.is_err() {
                                        break;
                                    }
                                }
                            },
                            Err(e) => {
                                // If there's an exit status, it makes sense that we may error when
                                // trying to read from its stdout pipe (likely been closed). In that
                                // case, don't emit an error.
                                let should_error = match child.wait() {
                                    Ok(exit_status) => !exit_status.success(),
                                    Err(_) => true,
                                };

                                if should_error {
                                    let _ = stdout_read_tx.send(Ok(Value {
                                        value: UntaggedValue::Error(ShellError::labeled_error(
                                            format!("Unable to read from stdout ({})", e),
                                            "unable to read from stdout",
                                            &stdout_name_tag,
                                        )),
                                        tag: stdout_name_tag.clone(),
                                    }));
                                }

                                return Ok(());
                            }
                        }
                    }
                }

                // We can give an error when we see a non-zero exit code, but this is different
                // than what other shells will do.
                let external_failed = match child.wait() {
                    Err(_) => true,
                    Ok(exit_status) => !exit_status.success(),
                };

                if external_failed {
                    let cfg = nu_data::config::config(Tag::unknown());
                    if let Ok(cfg) = cfg {
                        if cfg.contains_key("nonzero_exit_errors") {
                            let _ = stdout_read_tx.send(Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    "External command failed",
                                    "command failed",
                                    &stdout_name_tag,
                                )),
                                tag: stdout_name_tag.clone(),
                            }));
                        }
                    }
                    let _ = stdout_read_tx.send(Ok(Value {
                        value: UntaggedValue::nothing(),
                        tag: stdout_name_tag,
                    }));
                }

                Ok(())
            });

            let stream = ChannelReceiver::new(rx);
            Ok(stream.into_input_stream())
        }
        Err(e) => Err(ShellError::labeled_error(
            format!("{}", e),
            "failed to spawn",
            &command.name_tag,
        )),
    }
}

struct ChannelReceiver {
    rx: Arc<Mutex<mpsc::Receiver<Result<Value, ShellError>>>>,
}

impl ChannelReceiver {
    pub fn new(rx: mpsc::Receiver<Result<Value, ShellError>>) -> Self {
        Self {
            rx: Arc::new(Mutex::new(rx)),
        }
    }
}

impl Iterator for ChannelReceiver {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let rx = self.rx.lock();
        match rx.recv() {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}

fn argument_is_quoted(argument: &str) -> bool {
    if argument.len() < 2 {
        return false;
    }

    (argument.starts_with('"') && argument.ends_with('"'))
        || (argument.starts_with('\'') && argument.ends_with('\''))
}

#[allow(unused)]
fn add_double_quotes(argument: &str) -> String {
    format!("\"{}\"", argument)
}

#[allow(unused)]
fn escape_double_quotes(argument: &str) -> Cow<'_, str> {
    // allocate new string only if required
    if argument.contains('"') {
        Cow::Owned(argument.replace('"', r#"\""#))
    } else {
        Cow::Borrowed(argument)
    }
}

#[allow(unused)]
fn remove_quotes(argument: &str) -> Option<&str> {
    if !argument_is_quoted(argument) {
        return None;
    }

    let size = argument.len();

    Some(&argument[1..size - 1])
}

#[allow(unused)]
fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

#[cfg(test)]
mod tests {
    use super::{add_double_quotes, argument_is_quoted, escape_double_quotes, remove_quotes};
    #[cfg(feature = "which")]
    use super::{run_external_command, InputStream};

    #[cfg(feature = "which")]
    use nu_engine::EvaluationContext;

    #[cfg(feature = "which")]
    use nu_test_support::commands::ExternalBuilder;
    // fn read(mut stream: OutputStream) -> Option<Value> {
    //     match stream.try_next() {
    //         Ok(val) => {
    //             if let Some(val) = val {
    //                 val.raw_value()
    //             } else {
    //                 None
    //             }
    //         }
    //         Err(_) => None,
    //     }
    // }

    #[cfg(feature = "which")]
    fn non_existent_run() {
        use nu_protocol::hir::ExternalRedirection;
        let cmd = ExternalBuilder::for_name("i_dont_exist.exe").build();

        let input = InputStream::empty();
        let mut ctx = EvaluationContext::basic();

        assert!(run_external_command(cmd, &mut ctx, input, ExternalRedirection::Stdout).is_err());
    }

    // fn failure_run() -> Result<(), ShellError> {
    //     let cmd = ExternalBuilder::for_name("fail").build();

    //     let mut ctx = crate::cli::EvaluationContext::basic().expect("There was a problem creating a basic context.");
    //     let stream = run_external_command(cmd, &mut ctx, None, false)
    //         ?
    //         .expect("There was a problem running the external command.");

    //     match read(stream.into()) {
    //         Some(Value {
    //             value: UntaggedValue::Error(_),
    //             ..
    //         }) => {}
    //         None | _ => panic!("Command didn't fail."),
    //     }

    //     Ok(())
    // }

    // #[test]
    // fn identifies_command_failed() -> Result<(), ShellError> {
    //     block_on(failure_run())
    // }

    #[cfg(feature = "which")]
    #[test]
    fn identifies_command_not_found() {
        non_existent_run()
    }

    #[test]
    fn checks_escape_double_quotes() {
        assert_eq!(escape_double_quotes("andrés"), "andrés");
        assert_eq!(escape_double_quotes(r#"an"drés"#), r#"an\"drés"#);
        assert_eq!(escape_double_quotes(r#""an"drés""#), r#"\"an\"drés\""#);
    }

    #[test]
    fn checks_quotes_from_argument_to_be_passed_in() {
        assert!(!argument_is_quoted(""));

        assert!(!argument_is_quoted("'"));
        assert!(!argument_is_quoted("'a"));
        assert!(!argument_is_quoted("a"));
        assert!(!argument_is_quoted("a'"));
        assert!(argument_is_quoted("''"));

        assert!(!argument_is_quoted(r#"""#));
        assert!(!argument_is_quoted(r#""a"#));
        assert!(!argument_is_quoted(r#"a"#));
        assert!(!argument_is_quoted(r#"a""#));
        assert!(argument_is_quoted(r#""""#));

        assert!(!argument_is_quoted("'andrés"));
        assert!(!argument_is_quoted("andrés'"));
        assert!(!argument_is_quoted(r#""andrés"#));
        assert!(!argument_is_quoted(r#"andrés""#));
        assert!(argument_is_quoted("'andrés'"));
        assert!(argument_is_quoted(r#""andrés""#));
    }

    #[test]
    fn adds_double_quotes_to_argument_to_be_passed_in() {
        assert_eq!(add_double_quotes("andrés"), "\"andrés\"");
    }

    #[test]
    fn strips_quotes_from_argument_to_be_passed_in() {
        assert_eq!(remove_quotes(""), None);

        assert_eq!(remove_quotes("'"), None);
        assert_eq!(remove_quotes("'a"), None);
        assert_eq!(remove_quotes("a"), None);
        assert_eq!(remove_quotes("a'"), None);
        assert_eq!(remove_quotes("''"), Some(""));

        assert_eq!(remove_quotes(r#"""#), None);
        assert_eq!(remove_quotes(r#""a"#), None);
        assert_eq!(remove_quotes(r#"a"#), None);
        assert_eq!(remove_quotes(r#"a""#), None);
        assert_eq!(remove_quotes(r#""""#), Some(""));

        assert_eq!(remove_quotes("'andrés"), None);
        assert_eq!(remove_quotes("andrés'"), None);
        assert_eq!(remove_quotes(r#""andrés"#), None);
        assert_eq!(remove_quotes(r#"andrés""#), None);
        assert_eq!(remove_quotes("'andrés'"), Some("andrés"));
        assert_eq!(remove_quotes(r#""andrés""#), Some("andrés"));
    }
}
