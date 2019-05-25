use crate::prelude::*;
use futures::TryStreamExt;
use futures_codec::{Encoder, Decoder, Framed};
use std::sync::Arc;
use subprocess::Exec;
use std::io::{Error, ErrorKind};
use bytes::{BufMut, BytesMut};

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
            _ => Ok(None)
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

crate enum ClassifiedCommand {
    Internal(InternalCommand),
    External(ExternalCommand),
}

crate struct InternalCommand {
    crate command: Arc<dyn Command>,
    crate args: Vec<Value>,
}

impl InternalCommand {
    crate async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
    ) -> Result<InputStream, ShellError> {
        let result = context.run_command(self.command, self.args, input.objects)?;
        let env = context.env.clone();

        let stream = result.filter_map(move |v| match v {
            ReturnValue::Action(action) => match action {
                CommandAction::ChangeCwd(cwd) => {
                    env.lock().unwrap().cwd = cwd;
                    futures::future::ready(None)
                }
            },

            ReturnValue::Value(v) => futures::future::ready(Some(v)),
        });

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
        let mut cmd = self.name.clone();
        for arg in self.args {
            cmd.push_str(" ");
            cmd.push_str(&arg);
        }
        let process = Exec::shell(&cmd)
            .cwd(context.env.lock().unwrap().cwd());

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
