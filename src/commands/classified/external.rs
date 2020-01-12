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

pub(crate) async fn run_external_command(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    trace!(target: "nu::run::external", "-> {}", command.name);

    let has_it_arg = command.args.iter().any(|arg| arg.contains("$it"));
    if has_it_arg {
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
    let name = command.name;
    let args = command.args;
    let name_tag = command.name_tag;
    let inputs = input.unwrap_or_else(InputStream::empty).into_vec().await;

    trace!(target: "nu::run::external", "inputs = {:?}", inputs);

    let input_strings = inputs
        .iter()
        .map(|i| match i {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            }
            | Value {
                value: UntaggedValue::Primitive(Primitive::Line(s)),
                ..
            } => Ok(s.clone()),
            _ => {
                let arg = args.iter().find(|arg| arg.contains("$it"));
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
                        &name_tag,
                    ))
                }
            }
        })
        .collect::<Result<Vec<String>, ShellError>>()?;

    let home_dir = dirs::home_dir();
    let commands = input_strings.iter().map(|i| {
        let args = args.iter().filter_map(|arg| {
            if arg.chars().all(|c| c.is_whitespace()) {
                None
            } else {
                let arg = shellexpand::tilde_with_context(arg.deref(), || home_dir.as_ref());
                Some(arg.replace("$it", &i))
            }
        });

        format!("{} {}", name, itertools::join(args, " "))
    });

    let mut process = Exec::shell(itertools::join(commands, " && "));

    process = process.cwd(context.shell_manager.path()?);
    trace!(target: "nu::run::external", "cwd = {:?}", context.shell_manager.path());

    if !is_last {
        process = process.stdout(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    trace!(target: "nu::run::external", "built process {:?}", process);

    let popen = process.detached().popen();
    if let Ok(mut popen) = popen {
        if is_last {
            let _ = popen.wait();
            Ok(None)
        } else {
            let stdout = popen.stdout.take().ok_or_else(|| {
                ShellError::untagged_runtime_error("Can't redirect the stdout for external command")
            })?;
            let file = futures::io::AllowStdIo::new(stdout);
            let stream = Framed::new(file, LinesCodec {});
            let stream = stream.map(move |line| {
                line.expect("Internal error: could not read lines of text from stdin")
                    .into_value(&name_tag)
            });
            Ok(Some(stream.boxed().into()))
        }
    } else {
        Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            name_tag,
        ))
    }
}

async fn run_with_stdin(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let name_tag = command.name_tag;
    let home_dir = dirs::home_dir();

    let mut process = Exec::cmd(&command.name);
    for arg in command.args.iter() {
        // Let's also replace ~ as we shell out
        let arg = shellexpand::tilde_with_context(arg.deref(), || home_dir.as_ref());

        // Strip quotes from a quoted string
        if arg.len() > 1
            && ((arg.starts_with('"') && arg.ends_with('"'))
                || (arg.starts_with('\'') && arg.ends_with('\'')))
        {
            process = process.arg(arg.chars().skip(1).take(arg.len() - 2).collect::<String>());
        } else {
            process = process.arg(arg.as_ref());
        }
    }

    process = process.cwd(context.shell_manager.path()?);
    trace!(target: "nu::run::external", "cwd = {:?}", context.shell_manager.path());

    if !is_last {
        process = process.stdout(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    if input.is_some() {
        process = process.stdin(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built process {:?}", process);

    let popen = process.detached().popen();
    if let Ok(mut popen) = popen {
        let stream = async_stream! {
            if let Some(mut input) = input {
                let mut stdin_write = popen
                    .stdin
                    .take()
                    .expect("Internal error: could not get stdin pipe for external command");

                while let Some(item) = input.next().await {
                    match item.value {
                        UntaggedValue::Primitive(Primitive::Nothing) => {
                            // If first in a pipeline, will receive Nothing. This is not an error.
                        },

                        UntaggedValue::Primitive(Primitive::String(s)) |
                            UntaggedValue::Primitive(Primitive::Line(s)) =>
                        {
                            if let Err(e) = stdin_write.write(s.as_bytes()) {
                                let message = format!("Unable to write to stdin (error = {})", e);
                                yield Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "application may have closed before completing pipeline",
                                        &name_tag,
                                    )),
                                    tag: name_tag,
                                });
                                return;
                            }
                        },

                        // TODO serialize other primitives? https://github.com/nushell/nushell/issues/778

                        v => {
                            let message = format!("Received unexpected type from pipeline ({})", v.type_name());
                            yield Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    message,
                                    "expected a string",
                                    &name_tag,
                                )),
                                tag: name_tag,
                            });
                            return;
                        },
                    }
                }

                // Close stdin, which informs the external process that there's no more input
                drop(stdin_write);
            }

            if !is_last {
                let stdout = if let Some(stdout) = popen.stdout.take() {
                    stdout
                } else {
                    yield Ok(Value {
                        value: UntaggedValue::Error(
                            ShellError::labeled_error(
                                "Can't redirect the stdout for external command",
                                "can't redirect stdout",
                                &name_tag,
                            )
                        ),
                        tag: name_tag,
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

            let errored = match popen.wait() {
                Ok(status) => !status.success(),
                Err(e) => true,
            };

            if errored {
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
            };
        };

        Ok(Some(stream.to_input_stream()))
    } else {
        Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            name_tag,
        ))
    }
}
