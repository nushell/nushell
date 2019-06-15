use crate::commands::command::Sink;
use crate::parser::ast::Expression;
use crate::parser::lexer::Span;
use crate::parser::registry::Args;
use crate::prelude::*;
use bytes::{BufMut, BytesMut};
use futures::stream::StreamExt;
use futures_codec::{Decoder, Encoder, Framed};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
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
            objects: VecDeque::new().boxed(),
            stdin: None,
        }
    }

    crate fn from_input_stream(stream: InputStream) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: stream,
            stdin: None,
        }
    }

    crate fn from_stdout(stdout: std::fs::File) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().boxed(),
            stdin: Some(stdout),
        }
    }
}

crate struct ClassifiedPipeline {
    crate commands: Vec<ClassifiedCommand>,
}

crate enum ClassifiedCommand {
    #[allow(unused)]
    Expr(Expression),
    Internal(InternalCommand),
    Sink(SinkCommand),
    External(ExternalCommand),
}

crate struct SinkCommand {
    crate command: Arc<dyn Sink>,
    crate name_span: Option<Span>,
    crate args: Args,
}

impl SinkCommand {
    crate fn run(self, context: &mut Context, input: Vec<Value>) -> Result<(), ShellError> {
        context.run_sink(self.command, self.name_span.clone(), self.args, input)
    }
}

crate struct InternalCommand {
    crate command: Arc<dyn Command>,
    crate name_span: Option<Span>,
    crate args: Args,
}

impl InternalCommand {
    crate async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
    ) -> Result<InputStream, ShellError> {
        let mut result = context.run_command(
            self.command,
            self.name_span.clone(),
            self.args,
            input.objects,
        )?;
        let mut stream = VecDeque::new();
        while let Some(item) = result.next().await {
            match item {
                ReturnValue::Value(Value::Error(err)) => {
                    return Err(*err);
                }
                ReturnValue::Action(action) => match action {
                    CommandAction::ChangePath(path) => {
                        context.env.lock().unwrap().back_mut().map(|x| {
                            x.path = path;
                            x
                        });
                    }
                    CommandAction::Enter(obj) => {
                        let new_env = Environment {
                            obj: obj,
                            path: PathBuf::from("/"),
                        };
                        context.env.lock().unwrap().push_back(new_env);
                    }
                    CommandAction::Exit => match context.env.lock().unwrap().pop_back() {
                        Some(Environment {
                            obj: Value::Filesystem,
                            ..
                        }) => std::process::exit(0),
                        None => std::process::exit(-1),
                        _ => {}
                    },
                },

                ReturnValue::Value(v) => {
                    stream.push_back(v);
                }
            }
        }
        /*
        let stream = result.filter_map(move |v| match v {
            //ReturnValue::Value(Value::Error(err)) => futures::future::err(Err(err)),
            ReturnValue::Action(action) => match action {
                CommandAction::ChangePath(path) => {
                    env.lock().unwrap().last_mut().map(|x| {
                        x.path = path;
                        x
                    });
                    futures::future::ready(None)
                }
                CommandAction::Enter(obj) => {
                    let new_env = Environment {
                        obj: obj,
                        path: PathBuf::from("/"),
                    };
                    env.lock().unwrap().push(new_env);
                    futures::future::ready(None)
                }
                CommandAction::Exit => {
                    let mut v = env.lock().unwrap();
                    if v.len() == 1 {
                        std::process::exit(0);
                    }
                    v.pop();
                    futures::future::ready(None)
                }
            },

            ReturnValue::Value(v) => futures::future::ready(Some(v)),
        });
        */
        Ok(stream.boxed() as InputStream)
    }
}

crate struct ExternalCommand {
    crate name: String,
    crate args: Vec<String>,
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
        let inputs: Vec<Value> = input.objects.collect().await;

        let mut arg_string = format!("{}", self.name);
        for arg in &self.args {
            arg_string.push_str(" ");
            arg_string.push_str(&arg);
        }

        let mut process;
        #[cfg(windows)]
        {
            process = Exec::shell(&self.name);

            if arg_string.contains("$it") {
                let mut first = true;
                for i in &inputs {
                    if !first {
                        process = process.arg("&&");
                        process = process.arg(&self.name);
                    } else {
                        first = false;
                    }

                    for arg in &self.args {
                        process = process.arg(&arg.replace("$it", &i.as_string().unwrap()));
                    }
                }
            } else {
                for arg in &self.args {
                    process = process.arg(arg);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let mut new_arg_string = self.name.to_string();

            if arg_string.contains("$it") {
                let mut first = true;
                for i in &inputs {
                    if !first {
                        new_arg_string.push_str("&&");
                        new_arg_string.push_str(&self.name);
                    } else {
                        first = false;
                    }

                    for arg in &self.args {
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
        process = process.cwd(context.env.lock().unwrap().front().unwrap().path());

        let mut process = match stream_next {
            StreamNext::Last => process,
            StreamNext::External | StreamNext::Internal => {
                process.stdout(subprocess::Redirection::Pipe)
            }
        };

        if let Some(stdin) = input.stdin {
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
                let stream = stream.map(|line| Value::string(line.unwrap()));
                Ok(ClassifiedInputStream::from_input_stream(stream.boxed()))
            }
        }
    }
}
