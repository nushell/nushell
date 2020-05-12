use crate::evaluate::evaluate_baseline_expr;
use crate::futures::ThreadedReceiver;
use crate::prelude::*;

use std::io::Write;
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use bytes::{BufMut, Bytes, BytesMut};
use futures::executor::block_on_stream;
// use futures::stream::StreamExt;
use futures_codec::FramedRead;
use log::trace;

use nu_errors::ShellError;
use nu_protocol::hir::ExternalCommand;
use nu_protocol::{Primitive, Scope, ShellTypeName, UntaggedValue, Value};
use nu_source::Tag;

pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}
pub struct MaybeTextCodec;

impl futures_codec::Encoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = std::io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            StringOrBinary::String(s) => {
                dst.reserve(s.len());
                dst.put(s.as_bytes());
                Ok(())
            }
            StringOrBinary::Binary(b) => {
                dst.reserve(b.len());
                dst.put(Bytes::from(b));
                Ok(())
            }
        }
    }
}

impl futures_codec::Decoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let v: Vec<u8> = src.to_vec();
        match String::from_utf8(v) {
            Ok(s) => {
                src.clear();
                if s.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(StringOrBinary::String(s)))
                }
            }
            Err(err) => {
                // Note: the longest UTF-8 character per Unicode spec is currently 6 bytes. If we fail somewhere earlier than the last 6 bytes,
                // we know that we're failing to understand the string encoding and not just seeing a partial character. When this happens, let's
                // fall back to assuming it's a binary buffer.
                if src.is_empty() {
                    Ok(None)
                } else if src.len() > 6 && (src.len() - err.utf8_error().valid_up_to() > 6) {
                    // Fall back to assuming binary
                    let buf = src.to_vec();
                    src.clear();
                    Ok(Some(StringOrBinary::Binary(buf)))
                } else {
                    // Looks like a utf-8 string, so let's assume that
                    let buf = src.split_to(err.utf8_error().valid_up_to() + 1);
                    String::from_utf8(buf.to_vec())
                        .map(|x| Some(StringOrBinary::String(x)))
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }
            }
        }
    }
}

pub(crate) async fn run_external_command(
    command: ExternalCommand,
    context: &mut Context,
    input: InputStream,
    scope: &Scope,
    is_last: bool,
) -> Result<InputStream, ShellError> {
    trace!(target: "nu::run::external", "-> {}", command.name);

    if !did_find_command(&command.name).await {
        return Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            &command.name_tag,
        ));
    }

    run_with_stdin(command, context, input, scope, is_last).await
}

async fn run_with_stdin(
    command: ExternalCommand,
    context: &mut Context,
    input: InputStream,
    scope: &Scope,
    is_last: bool,
) -> Result<InputStream, ShellError> {
    let path = context.shell_manager.path();

    let input = trace_stream!(target: "nu::trace_stream::external::stdin", "input" = input);

    let mut command_args = vec![];
    for arg in command.args.iter() {
        let value = evaluate_baseline_expr(arg, &context.registry, scope).await?;
        // Skip any arguments that don't really exist, treating them as optional
        // FIXME: we may want to preserve the gap in the future, though it's hard to say
        // what value we would put in its place.
        if value.value.is_none() {
            continue;
        }
        // Do the cleanup that we need to do on any argument going out:
        let trimmed_value_string = value.as_string()?.trim_end_matches('\n').to_string();

        let value_string;
        #[cfg(not(windows))]
        {
            value_string = trimmed_value_string
                .replace('$', "\\$")
                .replace('"', "\\\"")
                .to_string()
        }
        #[cfg(windows)]
        {
            value_string = trimmed_value_string
        }

        command_args.push(value_string);
    }

    let process_args = command_args
        .iter()
        .map(|arg| {
            let arg = expand_tilde(arg.deref(), dirs::home_dir);

            #[cfg(not(windows))]
            {
                if argument_contains_whitespace(&arg) && !argument_is_quoted(&arg) {
                    add_quotes(&arg)
                } else {
                    arg.as_ref().to_string()
                }
            }
            #[cfg(windows)]
            {
                if let Some(unquoted) = remove_quotes(&arg) {
                    unquoted.to_string()
                } else {
                    arg.as_ref().to_string()
                }
            }
        })
        .collect::<Vec<String>>();

    spawn(&command, &path, &process_args[..], input, is_last, scope)
}

fn spawn(
    command: &ExternalCommand,
    path: &str,
    args: &[String],
    input: InputStream,
    is_last: bool,
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
                let arg = arg.replace("|", "\\|");
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
    process.envs(scope.env.iter());

    // We want stdout regardless of what
    // we are doing ($it case or pipe stdin)
    if !is_last {
        process.stdout(Stdio::piped());
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    // open since we have some contents for stdin
    if !input.is_empty() {
        process.stdin(Stdio::piped());
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built command {:?}", process);

    // TODO Switch to async_std::process once it's stabilized
    if let Ok(mut child) = process.spawn() {
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

                for value in block_on_stream(input) {
                    match &value.value {
                        UntaggedValue::Primitive(Primitive::Nothing) => continue,
                        UntaggedValue::Primitive(Primitive::String(s))
                        | UntaggedValue::Primitive(Primitive::Line(s)) => {
                            if let Err(e) = stdin_write.write(s.as_bytes()) {
                                let message = format!("Unable to write to stdin (error = {})", e);

                                let _ = stdin_write_tx.send(Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "application may have closed before completing pipeline",
                                        &stdin_name_tag,
                                    )),
                                    tag: stdin_name_tag,
                                }));
                                return Err(());
                            }
                        }
                        UntaggedValue::Primitive(Primitive::Binary(b)) => {
                            if let Err(e) = stdin_write.write(b) {
                                let message = format!("Unable to write to stdin (error = {})", e);

                                let _ = stdin_write_tx.send(Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "application may have closed before completing pipeline",
                                        &stdin_name_tag,
                                    )),
                                    tag: stdin_name_tag,
                                }));
                                return Err(());
                            }
                        }
                        unsupported => {
                            let _ = stdin_write_tx.send(Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    format!(
                                        "Received unexpected type from pipeline ({})",
                                        unsupported.type_name()
                                    ),
                                    "expected a string",
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
            if !is_last {
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

                let file = futures::io::AllowStdIo::new(stdout);
                let stream = FramedRead::new(file, MaybeTextCodec);

                for line in block_on_stream(stream) {
                    match line {
                        Ok(line) => match line {
                            StringOrBinary::String(s) => {
                                let result = stdout_read_tx.send(Ok(Value {
                                    value: UntaggedValue::Primitive(Primitive::String(s.clone())),
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

            // We can give an error when we see a non-zero exit code, but this is different
            // than what other shells will do.
            let external_failed = match child.wait() {
                Err(_) => true,
                Ok(exit_status) => !exit_status.success(),
            };

            if external_failed {
                let cfg = crate::data::config::config(Tag::unknown());
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
                    value: UntaggedValue::Error(ShellError::external_non_zero()),
                    tag: stdout_name_tag,
                }));
            }

            Ok(())
        });

        let stream = ThreadedReceiver::new(rx);
        Ok(stream.to_input_stream())
    } else {
        Err(ShellError::labeled_error(
            "Failed to spawn process",
            "failed to spawn",
            &command.name_tag,
        ))
    }
}

async fn did_find_command(name: &str) -> bool {
    #[cfg(not(windows))]
    {
        which::which(name).is_ok()
    }

    #[cfg(windows)]
    {
        if which::which(name).is_ok() {
            true
        } else {
            let cmd_builtins = [
                "call", "cls", "color", "date", "dir", "echo", "find", "hostname", "pause",
                "start", "time", "title", "ver", "copy", "mkdir", "rename", "rd", "rmdir", "type",
                "mklink",
            ];

            cmd_builtins.contains(&name)
        }
    }
}

fn expand_tilde<SI: ?Sized, P, HD>(input: &SI, home_dir: HD) -> std::borrow::Cow<str>
where
    SI: AsRef<str>,
    P: AsRef<std::path::Path>,
    HD: FnOnce() -> Option<P>,
{
    shellexpand::tilde_with_context(input, home_dir)
}

#[allow(unused)]
pub fn argument_contains_whitespace(argument: &str) -> bool {
    argument.chars().any(|c| c.is_whitespace())
}

fn argument_is_quoted(argument: &str) -> bool {
    if argument.len() < 2 {
        return false;
    }

    (argument.starts_with('"') && argument.ends_with('"'))
        || (argument.starts_with('\'') && argument.ends_with('\''))
}

#[allow(unused)]
fn add_quotes(argument: &str) -> String {
    format!("\"{}\"", argument)
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

    if let Some(paths) = std::env::var_os("PATH") {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

#[cfg(test)]
mod tests {
    use super::{
        add_quotes, argument_contains_whitespace, argument_is_quoted, expand_tilde, remove_quotes,
        run_external_command, Context, InputStream,
    };
    use futures::executor::block_on;
    use nu_errors::ShellError;
    use nu_protocol::Scope;
    use nu_test_support::commands::ExternalBuilder;

    // async fn read(mut stream: OutputStream) -> Option<Value> {
    //     match stream.try_next().await {
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

    async fn non_existent_run() -> Result<(), ShellError> {
        let cmd = ExternalBuilder::for_name("i_dont_exist.exe").build();

        let input = InputStream::empty();
        let mut ctx = Context::basic().expect("There was a problem creating a basic context.");

        assert!(
            run_external_command(cmd, &mut ctx, input, &Scope::empty(), false)
                .await
                .is_err()
        );

        Ok(())
    }

    // async fn failure_run() -> Result<(), ShellError> {
    //     let cmd = ExternalBuilder::for_name("fail").build();

    //     let mut ctx = Context::basic().expect("There was a problem creating a basic context.");
    //     let stream = run_external_command(cmd, &mut ctx, None, false)
    //         .await?
    //         .expect("There was a problem running the external command.");

    //     match read(stream.into()).await {
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

    #[test]
    fn identifies_command_not_found() -> Result<(), ShellError> {
        block_on(non_existent_run())
    }

    #[test]
    fn checks_contains_whitespace_from_argument_to_be_passed_in() {
        assert_eq!(argument_contains_whitespace("andrés"), false);
        assert_eq!(argument_contains_whitespace("and rés"), true);
        assert_eq!(argument_contains_whitespace(r#"and\ rés"#), true);
    }

    #[test]
    fn checks_quotes_from_argument_to_be_passed_in() {
        assert_eq!(argument_is_quoted(""), false);

        assert_eq!(argument_is_quoted("'"), false);
        assert_eq!(argument_is_quoted("'a"), false);
        assert_eq!(argument_is_quoted("a"), false);
        assert_eq!(argument_is_quoted("a'"), false);
        assert_eq!(argument_is_quoted("''"), true);

        assert_eq!(argument_is_quoted(r#"""#), false);
        assert_eq!(argument_is_quoted(r#""a"#), false);
        assert_eq!(argument_is_quoted(r#"a"#), false);
        assert_eq!(argument_is_quoted(r#"a""#), false);
        assert_eq!(argument_is_quoted(r#""""#), true);

        assert_eq!(argument_is_quoted("'andrés"), false);
        assert_eq!(argument_is_quoted("andrés'"), false);
        assert_eq!(argument_is_quoted(r#""andrés"#), false);
        assert_eq!(argument_is_quoted(r#"andrés""#), false);
        assert_eq!(argument_is_quoted("'andrés'"), true);
        assert_eq!(argument_is_quoted(r#""andrés""#), true);
    }

    #[test]
    fn adds_quotes_to_argument_to_be_passed_in() {
        assert_eq!(add_quotes("andrés"), "\"andrés\"");
        //assert_eq!(add_quotes("\"andrés\""), "\"andrés\"");
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

    #[test]
    fn expands_tilde_if_starts_with_tilde_character() {
        assert_eq!(
            expand_tilde("~", || Some(std::path::Path::new("the_path_to_nu_light"))),
            "the_path_to_nu_light"
        );
    }

    #[test]
    fn does_not_expand_tilde_if_tilde_is_not_first_character() {
        assert_eq!(
            expand_tilde("1~1", || Some(std::path::Path::new("the_path_to_nu_light"))),
            "1~1"
        );
    }
}
