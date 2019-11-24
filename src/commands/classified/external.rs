use super::ClassifiedInputStream;
use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use itertools::Itertools;
use log::trace;
use std::fmt;
use std::io::{Error, ErrorKind};
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
    type Item = String;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match src.iter().position(|b| b == &b'\n') {
            Some(pos) if !src.is_empty() => {
                let buf = src.split_to(pos + 1);
                String::from_utf8(buf.to_vec())
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            _ if !src.is_empty() => {
                let drained = src.take();
                String::from_utf8(drained.to_vec())
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Command {
    pub(crate) name: String,

    pub(crate) name_tag: Tag,
    pub(crate) args: Spanned<Vec<Tagged<String>>>,
}

impl FormatDebug for Command {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.name)?;

        if self.args.item.len() > 0 {
            write!(f, " ")?;
            write!(f, "{}", self.args.iter().map(|i| i.debug(source)).join(" "))?;
        }

        Ok(())
    }
}

impl HasSpan for Command {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
}

#[derive(Debug)]
pub(crate) enum StreamNext {
    Last,
    External,
    Internal,
}

impl Command {
    pub(crate) async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        stream_next: StreamNext,
    ) -> Result<ClassifiedInputStream, ShellError> {
        let stdin = input.stdin;
        let inputs: Vec<Tagged<Value>> = input.objects.into_vec().await;

        trace!(target: "nu::run::external", "-> {}", self.name);
        trace!(target: "nu::run::external", "inputs = {:?}", inputs);

        let mut arg_string = format!("{}", self.name);
        for arg in &self.args.item {
            arg_string.push_str(&arg);
        }

        trace!(target: "nu::run::external", "command = {:?}", self.name);

        let mut process;
        if arg_string.contains("$it") {
            let input_strings = inputs
                .iter()
                .map(|i| {
                    i.as_string().map_err(|_| {
                        let arg = self.args.iter().find(|arg| arg.item.contains("$it"));
                        if let Some(arg) = arg {
                            ShellError::labeled_error(
                                "External $it needs string data",
                                "given row instead of string data",
                                arg.tag(),
                            )
                        } else {
                            ShellError::labeled_error(
                                "$it needs string data",
                                "given something else",
                                self.name_tag.clone(),
                            )
                        }
                    })
                })
                .collect::<Result<Vec<String>, ShellError>>()?;

            let commands = input_strings.iter().map(|i| {
                let args = self.args.iter().filter_map(|arg| {
                    if arg.chars().all(|c| c.is_whitespace()) {
                        None
                    } else {
                        Some(arg.replace("$it", &i))
                    }
                });

                format!("{} {}", self.name, itertools::join(args, " "))
            });

            process = Exec::shell(itertools::join(commands, " && "))
        } else {
            process = Exec::cmd(&self.name);
            for arg in &self.args.item {
                let arg_chars: Vec<_> = arg.chars().collect();
                if arg_chars.len() > 1
                    && arg_chars[0] == '"'
                    && arg_chars[arg_chars.len() - 1] == '"'
                {
                    // quoted string
                    let new_arg: String = arg_chars[1..arg_chars.len() - 1].iter().collect();
                    process = process.arg(new_arg);
                } else {
                    process = process.arg(arg.item.clone());
                }
            }
        }

        process = process.cwd(context.shell_manager.path());

        trace!(target: "nu::run::external", "cwd = {:?}", context.shell_manager.path());

        let mut process = match stream_next {
            StreamNext::Last => process,
            StreamNext::External | StreamNext::Internal => {
                process.stdout(subprocess::Redirection::Pipe)
            }
        };

        trace!(target: "nu::run::external", "set up stdout pipe");

        if let Some(stdin) = stdin {
            process = process.stdin(stdin);
        }

        trace!(target: "nu::run::external", "set up stdin pipe");
        trace!(target: "nu::run::external", "built process {:?}", process);

        let popen = process.popen();

        trace!(target: "nu::run::external", "next = {:?}", stream_next);

        let name_tag = self.name_tag.clone();
        if let Ok(mut popen) = popen {
            match stream_next {
                StreamNext::Last => {
                    let _ = popen.detach();
                    loop {
                        match popen.poll() {
                            None => {
                                let _ = std::thread::sleep(std::time::Duration::new(0, 100000000));
                            }
                            _ => {
                                let _ = popen.terminate();
                                break;
                            }
                        }
                    }
                    Ok(ClassifiedInputStream::new())
                }
                StreamNext::External => {
                    let _ = popen.detach();
                    let stdout = popen.stdout.take().unwrap();
                    Ok(ClassifiedInputStream::from_stdout(stdout))
                }
                StreamNext::Internal => {
                    let _ = popen.detach();
                    let stdout = popen.stdout.take().unwrap();
                    let file = futures::io::AllowStdIo::new(stdout);
                    let stream = Framed::new(file, LinesCodec {});
                    let stream =
                        stream.map(move |line| Value::string(line.unwrap()).tagged(&name_tag));
                    Ok(ClassifiedInputStream::from_input_stream(
                        stream.boxed() as BoxStream<'static, Tagged<Value>>
                    ))
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                "Command not found",
                "command not found",
                name_tag,
            ));
        }
    }
}
