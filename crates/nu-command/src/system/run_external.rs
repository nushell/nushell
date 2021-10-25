use std::borrow::Cow;
use std::cell::RefCell;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command as CommandSys, Stdio};
use std::rc::Rc;
use std::sync::mpsc;

use nu_protocol::{
    ast::{Call, Expression},
    engine::{Command, EvaluationContext},
    ShellError, Signature, SyntaxShape, Value,
};
use nu_protocol::{IntoPipelineData, PipelineData, Span, ValueStream};

use nu_engine::eval_expression;

const OUTPUT_BUFFER_SIZE: usize = 8192;

#[derive(Clone)]
pub struct External;

impl Command for External {
    fn name(&self) -> &str {
        "run_external"
    }

    fn usage(&self) -> &str {
        "Runs external command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("run_external")
            .switch("last_expression", "last_expression", None)
            .rest("rest", SyntaxShape::Any, "external command to run")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let command = ExternalCommand::try_new(call, context)?;
        command.run_with_input(input)
    }
}

pub struct ExternalCommand<'call, 'contex> {
    pub name: &'call Expression,
    pub args: &'call [Expression],
    pub context: &'contex EvaluationContext,
    pub last_expression: bool,
}

impl<'call, 'contex> ExternalCommand<'call, 'contex> {
    pub fn try_new(
        call: &'call Call,
        context: &'contex EvaluationContext,
    ) -> Result<Self, ShellError> {
        if call.positional.is_empty() {
            return Err(ShellError::ExternalNotSupported(call.head));
        }

        Ok(Self {
            name: &call.positional[0],
            args: &call.positional[1..],
            context,
            last_expression: call.has_flag("last_expression"),
        })
    }

    pub fn get_name(&self) -> Result<String, ShellError> {
        let value = eval_expression(self.context, self.name)?;
        value.as_string()
    }

    pub fn get_args(&self) -> Vec<String> {
        self.args
            .iter()
            .filter_map(|expr| eval_expression(self.context, expr).ok())
            .filter_map(|value| value.as_string().ok())
            .collect()
    }

    pub fn run_with_input(&self, input: PipelineData) -> Result<PipelineData, ShellError> {
        let mut process = self.create_command();

        // TODO. We don't have a way to know the current directory
        // This should be information from the EvaluationContex or EngineState
        let path = env::current_dir().unwrap();
        process.current_dir(path);

        let envs = self.context.stack.get_env_vars();
        process.envs(envs);

        // If the external is not the last command, its output will get piped
        // either as a string or binary
        if !self.last_expression {
            process.stdout(Stdio::piped());
        }

        // If there is an input from the pipeline. The stdin from the process
        // is piped so it can be used to send the input information
        if let PipelineData::Value(Value::String { .. }) = input {
            process.stdin(Stdio::piped());
        }

        if let PipelineData::Stream { .. } = input {
            process.stdin(Stdio::piped());
        }

        match process.spawn() {
            Err(err) => Err(ShellError::ExternalCommand(
                format!("{}", err),
                self.name.span,
            )),
            Ok(mut child) => {
                // if there is a string or a stream, that is sent to the pipe std
                if let Some(mut stdin_write) = child.stdin.take() {
                    for value in input {
                        match value {
                            Value::String { val, span: _ } => {
                                self.write_to_stdin(&mut stdin_write, val.as_bytes())?
                            }
                            Value::Binary { val, span: _ } => {
                                self.write_to_stdin(&mut stdin_write, &val)?
                            }
                            _ => continue,
                        }
                    }
                }

                // If this external is not the last expression, then its output is piped to a channel
                // and we create a ValueStream that can be consumed
                let value = if !self.last_expression {
                    let (tx, rx) = mpsc::channel();
                    let stdout = child.stdout.take().ok_or_else(|| {
                        ShellError::ExternalCommand(
                            "Error taking stdout from external".to_string(),
                            self.name.span,
                        )
                    })?;

                    std::thread::spawn(move || {
                        // Stdout is read using the Buffer reader. It will do so until there is an
                        // error or there are no more bytes to read
                        let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, stdout);
                        while let Ok(bytes) = buf_read.fill_buf() {
                            if bytes.is_empty() {
                                break;
                            }

                            // The Cow generated from the function represents the conversion
                            // from bytes to String. If no replacements are required, then the
                            // borrowed value is a proper UTF-8 string. The Owned option represents
                            // a string where the values had to be replaced, thus marking it as bytes
                            let data = match String::from_utf8_lossy(bytes) {
                                Cow::Borrowed(s) => Data::String(s.into()),
                                Cow::Owned(_) => Data::Bytes(bytes.to_vec()),
                            };

                            let length = bytes.len();
                            buf_read.consume(length);

                            match tx.send(data) {
                                Ok(_) => continue,
                                Err(_) => break,
                            }
                        }
                    });

                    // The ValueStream is consumed by the next expression in the pipeline
                    ChannelReceiver::new(rx).into_pipeline_data()
                } else {
                    PipelineData::new()
                };

                match child.wait() {
                    Err(err) => Err(ShellError::ExternalCommand(
                        format!("{}", err),
                        self.name.span,
                    )),
                    Ok(_) => Ok(value),
                }
            }
        }
    }

    fn create_command(&self) -> CommandSys {
        // in all the other cases shell out
        if cfg!(windows) {
            //TODO. This should be modifiable from the config file.
            // We could give the option to call from powershell
            // for minimal builds cwd is unused
            let mut process = CommandSys::new("cmd");
            process.arg("/c");
            process.arg(&self.get_name().unwrap());
            for arg in self.get_args() {
                // Clean the args before we use them:
                // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
                // cmd.exe needs to have a caret to escape a pipe
                let arg = arg.replace("|", "^|");
                process.arg(&arg);
            }
            process
        } else {
            let cmd_with_args = vec![self.get_name().unwrap(), self.get_args().join(" ")].join(" ");
            let mut process = CommandSys::new("sh");
            process.arg("-c").arg(cmd_with_args);
            process
        }
    }

    fn write_to_stdin(&self, stdin_write: &mut ChildStdin, val: &[u8]) -> Result<(), ShellError> {
        if stdin_write.write(val).is_err() {
            Err(ShellError::ExternalCommand(
                "Error writing input to stdin".to_string(),
                self.name.span,
            ))
        } else {
            Ok(())
        }
    }
}

// The piped data from stdout from the external command can be either String
// or binary. We use this enum to pass the data from the spawned process
enum Data {
    String(String),
    Bytes(Vec<u8>),
}

// Receiver used for the ValueStream
// It implements iterator so it can be used as a ValueStream
struct ChannelReceiver {
    rx: mpsc::Receiver<Data>,
}

impl ChannelReceiver {
    pub fn new(rx: mpsc::Receiver<Data>) -> Self {
        Self { rx }
    }
}

impl Iterator for ChannelReceiver {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Ok(v) => match v {
                Data::String(s) => Some(Value::String {
                    val: s,
                    span: Span::unknown(),
                }),
                Data::Bytes(b) => Some(Value::Binary {
                    val: b,
                    span: Span::unknown(),
                }),
            },
            Err(_) => None,
        }
    }
}
