use crate::parser::{hir, TokenNode};
use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use derive_new::new;
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use log::{log_enabled, trace};
use nu_source::PrettyDebug;
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
            objects: vec![UntaggedValue::nothing().into_value(Tag::unknown())].into(),
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

#[derive(Debug, Clone)]
pub struct Commands {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl std::ops::Deref for Commands {
    type Target = [ClassifiedCommand];

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClassifiedPipeline {
    pub commands: Commands,
}

impl ClassifiedPipeline {
    pub fn commands(list: Vec<ClassifiedCommand>, span: impl Into<Span>) -> ClassifiedPipeline {
        ClassifiedPipeline {
            commands: Commands {
                list,
                span: span.into(),
            },
        }
    }
}

impl PrettyDebugWithSource for ClassifiedPipeline {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.commands.iter().map(|c| c.pretty_debug(source)),
            b::operator(" | "),
        )
        .or(b::delimit("<", b::description("empty pipeline"), ">"))
    }
}

impl HasSpan for ClassifiedPipeline {
    fn span(&self) -> Span {
        self.commands.span
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    #[allow(unused)]
    Dynamic(hir::Call),
    Internal(InternalCommand),
    External(ExternalCommand),
}

impl PrettyDebugWithSource for ClassifiedCommand {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            ClassifiedCommand::Expr(token) => b::typed("command", token.pretty_debug(source)),
            ClassifiedCommand::Dynamic(call) => b::typed("command", call.pretty_debug(source)),
            ClassifiedCommand::Internal(internal) => internal.pretty_debug(source),
            ClassifiedCommand::External(external) => external.pretty_debug(source),
        }
    }
}

impl HasSpan for ClassifiedCommand {
    fn span(&self) -> Span {
        match self {
            ClassifiedCommand::Expr(node) => node.span(),
            ClassifiedCommand::Internal(command) => command.span(),
            ClassifiedCommand::Dynamic(call) => call.span,
            ClassifiedCommand::External(command) => command.span(),
        }
    }
}

#[derive(new, Debug, Clone, Eq, PartialEq)]
pub struct InternalCommand {
    pub(crate) name: String,
    pub(crate) name_tag: Tag,
    pub(crate) args: hir::Call,
}

impl PrettyDebugWithSource for InternalCommand {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "internal command",
            b::description(&self.name) + b::space() + self.args.pretty_debug(source),
        )
    }
}

impl HasSpan for InternalCommand {
    fn span(&self) -> Span {
        let start = self.name_tag.span;

        start.until(self.args.span)
    }
}

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct DynamicCommand {
    pub(crate) args: hir::Call,
}

impl InternalCommand {
    pub(crate) fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        source: Text,
    ) -> Result<InputStream, ShellError> {
        if log_enabled!(log::Level::Trace) {
            trace!(target: "nu::run::internal", "->");
            trace!(target: "nu::run::internal", "{}", self.name);
            trace!(target: "nu::run::internal", "{}", self.args.debug(&source));
        }

        let objects: InputStream =
            trace_stream!(target: "nu::trace_stream::internal", "input" = input.objects);

        let command = context.expect_command(&self.name);

        let result =
            { context.run_command(command, self.name_tag.clone(), self.args, &source, objects) };

        let result = trace_out_stream!(target: "nu::trace_stream::internal", "output" = result);
        let mut result = result.values;
        let mut context = context.clone();

        let stream = async_stream! {
            let mut soft_errs: Vec<ShellError> = vec![];
            let mut yielded = false;

            while let Some(item) = result.next().await {
                match item {
                    Ok(ReturnSuccess::Action(action)) => match action {
                        CommandAction::ChangePath(path) => {
                            context.shell_manager.set_path(path);
                        }
                        CommandAction::Exit => std::process::exit(0), // TODO: save history.txt
                        CommandAction::Error(err) => {
                            context.error(err);
                            break;
                        }
                        CommandAction::EnterHelpShell(value) => {
                            match value {
                                Value {
                                    value: UntaggedValue::Primitive(Primitive::String(cmd)),
                                    tag,
                                } => {
                                    context.shell_manager.insert_at_current(Box::new(
                                        HelpShell::for_command(
                                            UntaggedValue::string(cmd).into_value(tag),
                                            &context.registry(),
                                        ).unwrap(),
                                    ));
                                }
                                _ => {
                                    context.shell_manager.insert_at_current(Box::new(
                                        HelpShell::index(&context.registry()).unwrap(),
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
                                FilesystemShell::with_location(location, context.registry().clone()).unwrap(),
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

                    Ok(ReturnSuccess::Value(v)) => {
                        yielded = true;
                        yield Ok(v);
                    }

                    Ok(ReturnSuccess::DebugValue(v)) => {
                        yielded = true;

                        let doc = PrettyDebug::pretty_doc(&v);
                        let mut buffer = termcolor::Buffer::ansi();

                        doc.render_raw(
                            context.with_host(|host| host.width() - 5),
                            &mut crate::parser::debug::TermColored::new(&mut buffer),
                        ).unwrap();

                        let value = String::from_utf8_lossy(buffer.as_slice());

                        yield Ok(UntaggedValue::string(value).into_untagged_value())
                    }

                    Err(err) => {
                        context.error(err);
                        break;
                    }
                }
            }
        };

        Ok(stream.to_input_stream())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl std::ops::Deref for ExternalArg {
    type Target = str;

    fn deref(&self) -> &str {
        &self.arg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArgs {
    pub list: Vec<ExternalArg>,
    pub span: Span,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalArg> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [ExternalArg];

    fn deref(&self) -> &[ExternalArg] {
        &self.list
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalCommand {
    pub(crate) name: String,

    pub(crate) name_tag: Tag,
    pub(crate) args: ExternalArgs,
}

impl PrettyDebug for ExternalCommand {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed(
            "external command",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.args.iter().map(|a| b::primitive(format!("{}", a.arg))),
                        b::space(),
                    ),
                ),
        )
    }
}

impl HasSpan for ExternalCommand {
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

impl ExternalCommand {
    pub(crate) async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        stream_next: StreamNext,
    ) -> Result<ClassifiedInputStream, ShellError> {
        let stdin = input.stdin;
        let inputs: Vec<Value> = input.objects.into_vec().await;

        trace!(target: "nu::run::external", "-> {}", self.name);
        trace!(target: "nu::run::external", "inputs = {:?}", inputs);

        let mut arg_string = format!("{}", self.name);
        for arg in self.args.iter() {
            arg_string.push_str(&arg);
        }

        trace!(target: "nu::run::external", "command = {:?}", self.name);

        let mut process;
        if arg_string.contains("$it") {
            let input_strings = inputs
                .iter()
                .map(|i| {
                    i.as_string().map(|s| s.to_string()).map_err(|_| {
                        let arg = self.args.iter().find(|arg| arg.contains("$it"));
                        if let Some(arg) = arg {
                            ShellError::labeled_error(
                                "External $it needs string data",
                                "given row instead of string data",
                                &arg.tag,
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
            for arg in self.args.iter() {
                let arg_chars: Vec<_> = arg.chars().collect();
                if arg_chars.len() > 1
                    && arg_chars[0] == '"'
                    && arg_chars[arg_chars.len() - 1] == '"'
                {
                    // quoted string
                    let new_arg: String = arg_chars[1..arg_chars.len() - 1].iter().collect();
                    process = process.arg(new_arg);
                } else {
                    process = process.arg(arg.arg.clone());
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
                    let stream = stream.map(move |line| {
                        UntaggedValue::string(line.unwrap()).into_value(&name_tag)
                    });
                    Ok(ClassifiedInputStream::from_input_stream(
                        stream.boxed() as BoxStream<'static, Value>
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
