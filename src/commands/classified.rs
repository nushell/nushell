use crate::commands::Command;
use crate::parser::{hir, TokenNode};
use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use log::{log_enabled, trace};
use std::io::{Error, ErrorKind};
use std::sync::Arc;
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

pub(crate) enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    Internal(InternalCommand),
    External(ExternalCommand),
}

pub(crate) struct InternalCommand {
    pub(crate) command: Arc<Command>,
    pub(crate) name_span: Span,
    pub(crate) args: hir::Call,
}

impl InternalCommand {
    pub(crate) async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        source: Text,
    ) -> Result<InputStream, ShellError> {
        if log_enabled!(log::Level::Trace) {
            trace!(target: "nu::run::internal", "->");
            trace!(target: "nu::run::internal", "{}", self.command.name());
            trace!(target: "nu::run::internal", "{}", self.args.debug(&source));
        }

        let objects: InputStream =
            trace_stream!(target: "nu::trace_stream::internal", "input" = input.objects);

        let result = context.run_command(
            self.command,
            self.name_span.clone(),
            context.source_map.clone(),
            self.args,
            &source,
            objects,
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
                    CommandAction::AddSpanSource(uuid, span_source) => {
                        context.add_span_source(uuid, span_source);
                    }
                    CommandAction::Exit => std::process::exit(0),
                    CommandAction::EnterHelpShell(value) => {
                        match value {
                            Tagged {
                                item: Value::Primitive(Primitive::String(cmd)),
                                ..
                            } => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::for_command(
                                        Tagged::from_simple_spanned_item(
                                            Value::string(cmd),
                                            Span::unknown(),
                                        ),
                                        &context.registry().clone(),
                                    )?,
                                ));
                            }
                            _ => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::index(&context.registry().clone())?,
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
                            std::process::exit(0);
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

pub(crate) struct ExternalCommand {
    pub(crate) name: String,

    pub(crate) name_span: Span,
    pub(crate) args: Vec<Tagged<String>>,
}

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
        let name_span = self.name_span.clone();

        trace!(target: "nu::run::external", "-> {}", self.name);
        trace!(target: "nu::run::external", "inputs = {:?}", inputs);

        let mut arg_string = format!("{}", self.name);
        for arg in &self.args {
            arg_string.push_str(&arg);
        }

        let mut process;

        process = Exec::cmd(&self.name);

        if arg_string.contains("$it") {
            let mut first = true;

            for i in &inputs {
                if i.as_string().is_err() {
                    let mut span = None;
                    for arg in &self.args {
                        if arg.item.contains("$it") {
                            span = Some(arg.span());
                        }
                    }
                    if let Some(span) = span {
                        return Err(ShellError::labeled_error(
                            "External $it needs string data",
                            "given row instead of string data",
                            span,
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

        let mut process = match stream_next {
            StreamNext::Last => process,
            StreamNext::External | StreamNext::Internal => {
                process.stdout(subprocess::Redirection::Pipe)
            }
        };

        if let Some(stdin) = stdin {
            process = process.stdin(stdin);
        }

        let mut popen = process.popen()?;

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
                println!("");
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
                let stream = stream.map(move |line| {
                    Tagged::from_simple_spanned_item(Value::string(line.unwrap()), name_span)
                });
                Ok(ClassifiedInputStream::from_input_stream(
                    stream.boxed() as BoxStream<'static, Tagged<Value>>
                ))
            }
        }
    }
}
