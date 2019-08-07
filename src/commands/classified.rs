use crate::commands::command::Sink;
use crate::parser::{registry::Args, TokenNode};
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

crate struct ClassifiedInputStream {
    crate objects: InputStream,
    crate stdin: Option<std::fs::File>,
}

impl ClassifiedInputStream {
    crate fn new() -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().into(),
            stdin: None,
        }
    }

    crate fn from_input_stream(stream: impl Into<InputStream>) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: stream.into(),
            stdin: None,
        }
    }

    crate fn from_stdout(stdout: std::fs::File) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().into(),
            stdin: Some(stdout),
        }
    }
}

crate struct ClassifiedPipeline {
    crate commands: Vec<ClassifiedCommand>,
}

crate enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    Internal(InternalCommand),
    Sink(SinkCommand),
    External(ExternalCommand),
}

impl ClassifiedCommand {
    #[allow(unused)]
    pub fn span(&self) -> Span {
        match self {
            ClassifiedCommand::Expr(token) => token.span(),
            ClassifiedCommand::Internal(internal) => internal.name_span.into(),
            ClassifiedCommand::Sink(sink) => sink.name_span.into(),
            ClassifiedCommand::External(external) => external.name_span.into(),
        }
    }
}

crate struct SinkCommand {
    crate command: Arc<dyn Sink>,
    crate name_span: Span,
    crate args: Args,
}

impl SinkCommand {
    crate fn run(self, context: &mut Context, input: Vec<Tagged<Value>>) -> Result<(), ShellError> {
        context.run_sink(self.command, self.name_span.clone(), self.args, input)
    }
}

crate struct InternalCommand {
    crate command: Arc<dyn Command>,
    crate name_span: Span,
    crate args: Args,
}

impl InternalCommand {
    crate async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
    ) -> Result<InputStream, ShellError> {
        if log_enabled!(log::Level::Trace) {
            trace!(target: "nu::run::internal", "->");
            trace!(target: "nu::run::internal", "{}", self.command.name());
            trace!(target: "nu::run::internal", "{:?}", self.args.debug());
        }

        let objects: InputStream =
            trace_stream!(target: "nu::trace_stream::internal", "input" = input.objects);

        let result = context.run_command(
            self.command,
            self.name_span.clone(),
            context.source_map.clone(),
            self.args,
            objects,
        )?;

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
                    CommandAction::Enter(location) => {
                        let path = std::path::Path::new(&location);

                        if path.is_dir() {
                            // If it's a directory, add a new filesystem shell
                            context
                                .shell_manager
                                .push(Box::new(FilesystemShell::with_location(location)?));
                        } else {
                            // If it's a file, attempt to open the file as a value and enter it
                            let cwd = context.shell_manager.path();

                            let full_path = std::path::PathBuf::from(cwd);

                            let (file_extension, contents, contents_tag, _) =
                                crate::commands::open::fetch(
                                    &full_path,
                                    &location,
                                    Span::unknown(),
                                )?;

                            match contents {
                                Value::Primitive(Primitive::String(string)) => {
                                    let value = crate::commands::open::parse_as_value(
                                        file_extension,
                                        string,
                                        contents_tag,
                                        Span::unknown(),
                                    )?;

                                    context.shell_manager.push(Box::new(ValueShell::new(value)));
                                }
                                value => context
                                    .shell_manager
                                    .push(Box::new(ValueShell::new(value.tagged(Tag::unknown())))),
                            }
                        }
                    }
                    CommandAction::PreviousShell => {
                        context.shell_manager.prev();
                    }
                    CommandAction::NextShell => {
                        context.shell_manager.next();
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

crate struct ExternalCommand {
    crate name: String,
    #[allow(unused)]
    crate name_span: Span,
    crate args: Vec<Tagged<String>>,
}

crate enum StreamNext {
    Last,
    External,
    Internal,
}

impl ExternalCommand {
    crate async fn run(
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

        #[cfg(windows)]
        {
            process = Exec::shell(&self.name);

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
                                "given object instead of string data",
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

                        process = process.arg(&arg.replace("$it", &i.as_string().unwrap()));
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
        }
        #[cfg(not(windows))]
        {
            let mut new_arg_string = self.name.to_string();

            if arg_string.contains("$it") {
                let mut first = true;
                for i in &inputs {
                    if i.as_string().is_err() {
                        let mut span = name_span;
                        for arg in &self.args {
                            if arg.item.contains("$it") {
                                span = arg.span();
                            }
                        }
                        return Err(ShellError::labeled_error(
                            "External $it needs string data",
                            "given object instead of string data",
                            span,
                        ));
                    }
                    if !first {
                        new_arg_string.push_str("&&");
                        new_arg_string.push_str(&self.name);
                    } else {
                        first = false;
                    }

                    for arg in &self.args {
                        if arg.chars().all(|c| c.is_whitespace()) {
                            continue;
                        }

                        new_arg_string.push_str(" ");
                        new_arg_string.push_str(&arg.replace("$it", &i.as_string().unwrap()));
                    }
                }
            } else {
                for arg in &self.args {
                    new_arg_string.push_str(" ");
                    new_arg_string.push_str(&arg);
                }
            }

            process = Exec::shell(new_arg_string);
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

        let mut popen = process.popen().unwrap();

        match stream_next {
            StreamNext::Last => {
                popen.wait()?;
                Ok(ClassifiedInputStream::new())
            }
            StreamNext::External => {
                let stdout = popen.stdout.take().unwrap();
                Ok(ClassifiedInputStream::from_stdout(stdout))
            }
            StreamNext::Internal => {
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
