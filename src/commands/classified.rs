use crate::parser::{hir, TokenNode};
use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use derive_new::new;
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use log::{log_enabled, trace};
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

pub(crate) struct ClassifiedInputStream {
    pub(crate) objects: InputStream,
    pub(crate) stdin: Option<std::fs::File>,
}

impl ClassifiedInputStream {
    pub(crate) fn new() -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().into(),
            stdin: None,
        }
    }

    pub(crate) fn from_input_stream(stream: impl Into<InputStream>) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: stream.into(),
            stdin: None,
        }
    }

    pub(crate) fn from_stdout(stdout: std::fs::File) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().into(),
            stdin: Some(stdout),
        }
    }
}

pub(crate) struct ClassifiedPipeline {
    pub(crate) commands: Vec<ClassifiedCommand>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    Internal(InternalCommand),
    #[allow(unused)]
    Dynamic(hir::Call),
    External(ExternalCommand),
}

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct InternalCommand {
    pub(crate) name: String,
    pub(crate) name_tag: Tag,
    pub(crate) args: hir::Call,
}

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct DynamicCommand {
    pub(crate) args: hir::Call,
}

impl InternalCommand {
    pub(crate) async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        source: Text,
        is_first_command: bool,
    ) -> Result<InputStream, ShellError> {
        if log_enabled!(log::Level::Trace) {
            trace!(target: "nu::run::internal", "->");
            trace!(target: "nu::run::internal", "{}", self.name);
            trace!(target: "nu::run::internal", "{}", self.args.debug(&source));
        }

        let objects: InputStream =
            trace_stream!(target: "nu::trace_stream::internal", "input" = input.objects);

        let command = context.expect_command(&self.name);

        let result = context.run_command(
            command,
            self.name_tag.clone(),
            context.source_map.clone(),
            self.args,
            &source,
            objects,
            is_first_command,
        );

        let result = trace_out_stream!(target: "nu::trace_stream::internal", source: &source, "output" = result);
        let mut result = result.values;

        let mut stream = VecDeque::new();
        while let Some(item) = result.next().await {
            match item? {
                ReturnSuccess::Action(action) => match action {
                    CommandAction::ChangePath(path) => {
                        context.shell_manager.set_path(path);
                    }
                    CommandAction::AddAnchorLocation(uuid, anchor_location) => {
                        context.add_anchor_location(uuid, anchor_location);
                    }
                    CommandAction::Exit => std::process::exit(0), // TODO: save history.txt
                    CommandAction::EnterHelpShell(value) => {
                        match value {
                            Tagged {
                                item: Value::Primitive(Primitive::String(cmd)),
                                tag,
                            } => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::for_command(
                                        Value::string(cmd).tagged(tag),
                                        &context.registry(),
                                    )?,
                                ));
                            }
                            _ => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::index(&context.registry())?,
                                ));
                            }
                        }
                    }
                    CommandAction::EnterValueShell(value) => {
                        context
                            .shell_manager
                            .insert_at_current(Box::new(ValueShell::new(value)));
                    }
                    CommandAction::EnterShell(location) => {
                        context.shell_manager.insert_at_current(Box::new(
                            FilesystemShell::with_location(location, context.registry().clone())?,
                        ));
                    }
                    CommandAction::PreviousShell => {
                        context.shell_manager.prev();
                    }
                    CommandAction::NextShell => {
                        context.shell_manager.next();
                    }
                    CommandAction::LeaveShell => {
                        context.shell_manager.remove_at_current();
                        if context.shell_manager.is_empty() {
                            std::process::exit(0); // TODO: save history.txt
                        }
                    }
                },

                ReturnSuccess::Value(v) => {
                    stream.push_back(v);
                }
            }
        }

        Ok(stream.into())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ExternalCommand {
    pub(crate) name: String,

    pub(crate) name_tag: Tag,
    pub(crate) args: Vec<Tagged<String>>,
}

#[derive(Debug)]
pub(crate) enum StreamNext {
    Last,
    External,
    Internal,
}

impl ExternalCommand {
    pub(crate) async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        stream_next: StreamNext,
    ) -> Result<ClassifiedInputStream, ShellError> {
        let stdin = input.stdin;
        let inputs: Vec<Tagged<Value>> = input.objects.into_vec().await;
        let name_tag = self.name_tag.clone();

        trace!(target: "nu::run::external", "-> {}", self.name);
        trace!(target: "nu::run::external", "inputs = {:?}", inputs);

        let mut arg_string = format!("{}", self.name);
        for arg in &self.args {
            arg_string.push_str(&arg);
        }

        let mut process;

        process = Exec::cmd(&self.name);

        trace!(target: "nu::run::external", "command = {:?}", process);

        if arg_string.contains("$it") {
            let mut first = true;

            for i in &inputs {
                if i.as_string().is_err() {
                    let mut tag = None;
                    for arg in &self.args {
                        if arg.item.contains("$it") {
                            tag = Some(arg.tag());
                        }
                    }
                    if let Some(tag) = tag {
                        return Err(ShellError::labeled_error(
                            "External $it needs string data",
                            "given row instead of string data",
                            tag,
                        ));
                    } else {
                        return Err(ShellError::string("Error: $it needs string data"));
                    }
                }
                if !first {
                    process = process.arg("&&");
                    process = process.arg(&self.name);
                } else {
                    first = false;
                }

                for arg in &self.args {
                    if arg.chars().all(|c| c.is_whitespace()) {
                        continue;
                    }

                    process = process.arg(&arg.replace("$it", &i.as_string()?));
                }
            }
        } else {
            for arg in &self.args {
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

        let mut popen = process.popen().unwrap();

        trace!(target: "nu::run::external", "next = {:?}", stream_next);

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
                let stream = stream.map(move |line| Value::string(line.unwrap()).tagged(name_tag));
                Ok(ClassifiedInputStream::from_input_stream(
                    stream.boxed() as BoxStream<'static, Tagged<Value>>
                ))
            }
        }
    }
}
