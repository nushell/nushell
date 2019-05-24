use crate::prelude::*;
use futures::compat::AsyncRead01CompatExt;
use futures_codec::{Framed, LinesCodec};
use std::sync::Arc;
use subprocess::Exec;

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

impl ClassifiedCommand {
    crate async fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
    ) -> Result<InputStream, ShellError> {
        match self {
            ClassifiedCommand::Internal(internal) => {
                let result = context.run_command(internal.command, internal.args, input.objects)?;
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

            ClassifiedCommand::External(external) => {
                Exec::shell(&external.name)
                    .args(&external.args)
                    .cwd(context.env.lock().unwrap().cwd())
                    .join()
                    .unwrap();

                Ok(VecDeque::new().boxed() as InputStream)
            }
        }
    }
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
        mut input: ClassifiedInputStream,
        stream_next: StreamNext,
    ) -> Result<ClassifiedInputStream, ShellError> {
        let mut process = Exec::shell(&self.name)
            .args(&self.args)
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

        // if stream_next {
        //     let stdout = popen.stdout.take().unwrap();
        //     Ok(ClassifiedInputStream::from_stdout(stdout))
        // } else {
        //     // popen.stdin.take();
        //     popen.wait()?;
        //     Ok(ClassifiedInputStream::new())
        // }
    }
}
