use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use log::trace;
use nu_errors::ShellError;
use nu_parser::ExternalCommand;
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
use std::io::{Error, ErrorKind, Write};
use std::ops::Deref;
use subprocess::Exec;

/// A simple `Codec` implementation that splits up data into lines.
pub struct LinesCodec {}

impl Encoder for LinesCodec {
    type Item = String;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put(item);
        Ok(())
    }
}

impl Decoder for LinesCodec {
    type Item = nu_protocol::UntaggedValue;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match src.iter().position(|b| b == &b'\n') {
            Some(pos) if !src.is_empty() => {
                let buf = src.split_to(pos + 1);
                String::from_utf8(buf.to_vec())
                    .map(UntaggedValue::line)
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            _ if !src.is_empty() => {
                let drained = src.take();
                String::from_utf8(drained.to_vec())
                    .map(UntaggedValue::string)
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            _ => Ok(None),
        }
    }
}

fn argument_is_quoted(argument: &str) -> bool {
    if argument.len() < 2 {
        return false;
    }

    (argument.starts_with('"') && argument.ends_with('"')
        || (argument.starts_with('\'') && argument.ends_with('\'')))
}

fn remove_quotes(argument: &str) -> Option<&str> {
    if !argument_is_quoted(argument) {
        return None;
    }

    let size = argument.len();

    Some(&argument[1..size - 1])
}

fn expand_tilde<SI: ?Sized, P, HD>(input: &SI, home_dir: HD) -> std::borrow::Cow<str>
where
    SI: AsRef<str>,
    P: AsRef<std::path::Path>,
    HD: FnOnce() -> Option<P>,
{
    shellexpand::tilde_with_context(input, home_dir)
}

pub fn nu_value_to_string(command: &ExternalCommand, from: &Value) -> Result<String, ShellError> {
    match &from.value {
        UntaggedValue::Primitive(Primitive::Int(i)) => Ok(i.to_string()),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => Ok(s.clone()),
        UntaggedValue::Primitive(Primitive::Path(p)) => Ok(p.to_string_lossy().to_string()),
        _ => {
            let arg = command.expect_arg("$it");

            if let Some(arg) = arg {
                Err(ShellError::labeled_error(
                    "External $it needs string data",
                    "given row instead of string data",
                    &arg.tag,
                ))
            } else {
                Err(ShellError::labeled_error(
                    "$it needs string data",
                    "given something else",
                    &command.name_tag,
                ))
            }
        }
    }
}

// At the moment, when we pipe to an
// external that expects data from stdin
// we currently pipe all the rows to it
// as a single input for it.
pub fn fold_nu_values_for_stdin(
    command: &ExternalCommand,
    values: &[Value],
) -> Result<Option<String>, ShellError> {
    let mut str_input = String::new();

    for item in values {
        match nu_value_to_string_for_stdin(&command, &item) {
            Ok(maybe) => {
                if let Some(converted) = maybe {
                    str_input.push_str(&converted);
                }
            }
            Err(reason) => return Err(reason),
        }
    }

    if !str_input.is_empty() {
        return Ok(Some(str_input.clone()));
    }

    Ok(None)
}

pub fn nu_value_to_string_for_stdin(
    command: &ExternalCommand,
    from: &Value,
) -> Result<Option<String>, ShellError> {
    match &from.value {
        UntaggedValue::Primitive(Primitive::Nothing) => Ok(None),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => Ok(Some(s.clone())),
        unsupported => Err(ShellError::labeled_error(
            format!(
                "Received unexpected type from pipeline ({})",
                unsupported.type_name()
            ),
            "expected a string",
            &command.name_tag,
        )),
    }
}

pub(crate) async fn run_external_command(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    trace!(target: "nu::run::external", "-> {}", command.name);

    if command.has("$it") {
        run_with_iterator_arg(command, context, input, is_last).await
    } else {
        run_with_stdin(command, context, input, is_last).await
    }
}

async fn run_with_iterator_arg(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let path = context.shell_manager.path()?;

    let mut inputs: InputStream = if let Some(input) = input {
        trace_stream!(target: "nu::trace_stream::external", "input" = input)
    } else {
        InputStream::empty()
    };

    let stream = async_stream! {
        while let Some(value) = inputs.next().await {
            let name = command.name.clone();
            let name_tag = command.name_tag.clone();
            let home_dir = dirs::home_dir();
            let path = &path;
            let args = command.args.clone();

            let it_replacement = match nu_value_to_string(&command, &value) {
                Ok(value) => value,
                Err(reason) => {
                    yield Ok(Value {
                        value: UntaggedValue::Error(reason),
                        tag: name_tag
                    });
                    return;
                }
            };

            let process_args = args.iter().filter_map(|arg| {
                if arg.chars().all(|c| c.is_whitespace()) {
                    None
                } else {
                    let arg = expand_tilde(arg.deref(), || home_dir.as_ref());
                    Some(arg.replace("$it", &it_replacement))
                }
            }).collect::<Vec<_>>();

            match spawn(&command, &path, &process_args, None, is_last).await {
                Ok(res) => {
                    if let Some(mut res) = res {
                        while let Some(item) = res.next().await {
                            yield Ok(item)
                        }
                    }
                }
                Err(reason) => {
                    yield Ok(Value {
                        value: UntaggedValue::Error(reason),
                        tag: name_tag
                    });
                    return;
                }
            }
        }
    };

    Ok(Some(stream.to_input_stream()))
}

async fn run_with_stdin(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let home_dir = dirs::home_dir();
    let path = context.shell_manager.path()?;
    let name_tag = command.name_tag.clone();
    let args = command.args.clone();

    let inputs = input.unwrap_or_else(InputStream::empty).into_vec().await;

    trace!(target: "nu::run::external", "inputs = {:?}", inputs);

    let stream = async_stream! {
        let collected_values_for_stdin = match fold_nu_values_for_stdin(&command, &inputs) {
            Ok(folded) => folded,
            Err(reason) => {
                yield Ok(Value {
                    value: UntaggedValue::Error(reason),
                    tag: name_tag
                });
                return;
            }
        };

        let process_args = args.iter().map(|arg| {
            let arg = expand_tilde(arg.deref(), || home_dir.as_ref());
            if let Some(unquoted) = remove_quotes(&arg) {
                unquoted.to_string()
            } else {
                arg.to_string()
            }
        }).collect::<Vec<_>>();

        match spawn(&command, &path, &process_args, collected_values_for_stdin, is_last).await {
            Ok(res) => {
                if let Some(mut res) = res {
                    while let Some(item) = res.next().await {
                        yield Ok(item)
                    }
                }
            }
            Err(reason) => {
                yield Ok(Value {
                    value: UntaggedValue::Error(reason),
                    tag: name_tag
                });
                return;
            }
        }
    };

    Ok(Some(stream.to_input_stream()))
}

async fn spawn(
    command: &ExternalCommand,
    path: &str,
    args: &[String],
    values: Option<String>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let name_tag = command.name_tag.clone();

    let mut process = Exec::cmd(&command.name);

    for arg in args {
        process = process.arg(&arg);
    }

    process = process.cwd(path);
    trace!(target: "nu::run::external", "cwd = {:?}", &path);

    if !is_last {
        process = process.stdout(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    if values.is_some() {
        process = process.stdin(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built process {:?}", process);

    let popen = process.detached().popen();

    if let Ok(mut popen) = popen {
        let stream = async_stream! {
            if let Some(mut input) = values {
                let mut stdin_write = popen.stdin
                    .take()
                    .expect("Internal error: could not get stdin pipe for external command");

                if let Err(e) = stdin_write.write(input.as_bytes()) {
                    let message = format!("Unable to write to stdin (error = {})", e);

                    yield Ok(Value {
                        value: UntaggedValue::Error(ShellError::labeled_error(
                            message,
                            "application may have closed before completing pipeline",
                            &name_tag)),
                        tag: name_tag
                    });
                    return;
                }

                drop(stdin_write);
            }

            if !is_last {
                let stdout = if let Some(stdout) = popen.stdout.take() {
                    stdout
                } else {
                    yield Ok(Value {
                        value: UntaggedValue::Error(ShellError::labeled_error(
                            "Can't redirect the stdout for external command",
                            "can't redirect stdout",
                            &name_tag)),
                        tag: name_tag
                    });
                    return;
                };

                let file = futures::io::AllowStdIo::new(stdout);
                let stream = Framed::new(file, LinesCodec {});

                let mut stream = stream.map(|line| {
                    if let Ok(line) = line {
                        line.into_value(&name_tag)
                    } else {
                        panic!("Internal error: could not read lines of text from stdin")
                    }
                });

                loop {
                    match stream.next().await {
                        Some(item) => yield Ok(item),
                        None => break,
                    }
                }
            }

            if is_last {
                if let Ok(status) = popen.wait() {
                    if status.success() {
                        return;
                    }
                }

                yield Ok(Value {
                    value: UntaggedValue::Error(ShellError::labeled_error(
                            "External command failed",
                            "command failed",
                            &name_tag)),
                    tag: name_tag
                });
                return;
            }

            loop {
                match popen.poll() {
                    None => std::thread::sleep(std::time::Duration::new(0, 100_000_000)),
                    Some(status) => {
                        if !status.success() {
                            yield Ok(Value {
                                value: UntaggedValue::Error(
                                    ShellError::labeled_error(
                                        "External command failed",
                                        "command failed",
                                        &name_tag,
                                    )
                                ),
                                tag: name_tag,
                            });
                        }
                        break;
                    }
                }
            }
        };
        Ok(Some(stream.to_input_stream()))
    } else {
        Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            &command.name_tag,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        argument_is_quoted, expand_tilde, remove_quotes, run_external_command, Context,
        OutputStream,
    };
    use futures::executor::block_on;
    use futures::stream::TryStreamExt;
    use nu_errors::ShellError;
    use nu_protocol::{UntaggedValue, Value};
    use nu_test_support::commands::ExternalBuilder;

    async fn read(mut stream: OutputStream) -> Option<Value> {
        match stream.try_next().await {
            Ok(val) => {
                if let Some(val) = val {
                    val.raw_value()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn non_existent_run() -> Result<(), ShellError> {
        let cmd = ExternalBuilder::for_name("i_dont_exist.exe").build();

        let mut ctx = Context::basic().expect("There was a problem creating a basic context.");

        let stream = run_external_command(cmd, &mut ctx, None, false)
            .await?
            .expect("There was a problem running the external command.");

        match read(stream.into()).await {
            Some(Value {
                value: UntaggedValue::Error(_),
                ..
            }) => {}
            None | _ => panic!("Apparently a command was found (It's not supposed to be found)"),
        }

        Ok(())
    }

    async fn failure_run() -> Result<(), ShellError> {
        let cmd = ExternalBuilder::for_name("fail").build();

        let mut ctx = Context::basic().expect("There was a problem creating a basic context.");
        let stream = run_external_command(cmd, &mut ctx, None, false)
            .await?
            .expect("There was a problem running the external command.");

        match read(stream.into()).await {
            Some(Value {
                value: UntaggedValue::Error(_),
                ..
            }) => {}
            None | _ => panic!("Command didn't fail."),
        }

        Ok(())
    }

    #[test]
    fn identifies_command_failed() -> Result<(), ShellError> {
        block_on(failure_run())
    }

    #[test]
    fn identifies_command_not_found() -> Result<(), ShellError> {
        block_on(non_existent_run())
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
