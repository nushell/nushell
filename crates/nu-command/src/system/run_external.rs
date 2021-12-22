use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command as CommandSys, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use nu_engine::env_to_strings;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{ast::Call, engine::Command, ShellError, Signature, SyntaxShape, Value};
use nu_protocol::{Category, Config, IntoInterruptiblePipelineData, PipelineData, Span, Spanned};

use itertools::Itertools;

use nu_engine::CallExt;
use regex::Regex;

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

    fn is_private(&self) -> bool {
        true
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("run_external")
            .switch("last_expression", "last_expression", None)
            .rest("rest", SyntaxShape::Any, "external command to run")
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let args: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let last_expression = call.has_flag("last_expression");

        // Translate environment variables from Values to Strings
        let config = stack.get_config().unwrap_or_default();
        let env_vars_str = env_to_strings(engine_state, stack, &config)?;

        // Check if this is a single call to a directory, if so auto-cd
        let path = nu_path::expand_path(&name.item);
        let orig = name.item.clone();
        name.item = path.to_string_lossy().to_string();

        let path = Path::new(&name.item);
        if (orig.starts_with('.')
            || orig.starts_with('~')
            || orig.starts_with('/')
            || orig.starts_with('\\'))
            && path.is_dir()
            && args.is_empty()
        {
            // We have an auto-cd
            let _ = std::env::set_current_dir(&path);

            //FIXME: this only changes the current scope, but instead this environment variable
            //should probably be a block that loads the information from the state in the overlay
            stack.add_env_var(
                "PWD".into(),
                Value::String {
                    val: name.item.clone(),
                    span: call.head,
                },
            );
            return Ok(PipelineData::new(call.head));
        }

        let mut args_strs = vec![];

        for arg in args {
            if let Ok(s) = arg.as_string() {
                args_strs.push(s);
            } else if let Value::List { vals, .. } = arg {
                // Interpret a list as a series of arguments
                for val in vals {
                    if let Ok(s) = val.as_string() {
                        args_strs.push(s);
                    } else {
                        return Err(ShellError::ExternalCommand(
                            "Cannot convert argument to a string".into(),
                            val.span()?,
                        ));
                    }
                }
            } else {
                return Err(ShellError::ExternalCommand(
                    "Cannot convert argument to a string".into(),
                    arg.span()?,
                ));
            }
        }

        let command = ExternalCommand {
            name,
            args: args_strs,
            last_expression,
            env_vars: env_vars_str,
            call,
        };
        command.run_with_input(engine_state, input, config)
    }
}

pub struct ExternalCommand<'call> {
    pub name: Spanned<String>,
    pub args: Vec<String>,
    pub last_expression: bool,
    pub env_vars: HashMap<String, String>,
    pub call: &'call Call,
}

impl<'call> ExternalCommand<'call> {
    pub fn run_with_input(
        &self,
        engine_state: &EngineState,
        input: PipelineData,
        config: Config,
    ) -> Result<PipelineData, ShellError> {
        let mut process = self.create_command();

        let ctrlc = engine_state.ctrlc.clone();

        // TODO. We don't have a way to know the current directory
        // This should be information from the EvaluationContex or EngineState
        let path = env::current_dir()?;

        process.current_dir(path);

        process.envs(&self.env_vars);

        // If the external is not the last command, its output will get piped
        // either as a string or binary
        if !self.last_expression {
            process.stdout(Stdio::piped());
        }

        // If there is an input from the pipeline. The stdin from the process
        // is piped so it can be used to send the input information
        if !matches!(input, PipelineData::Value(Value::Nothing { .. }, ..)) {
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
                    std::thread::spawn(move || {
                        for value in input.into_iter() {
                            match value {
                                Value::String { val, span: _ } => {
                                    if stdin_write.write(val.as_bytes()).is_err() {
                                        return Ok(());
                                    }
                                }
                                Value::Binary { val, span: _ } => {
                                    if stdin_write.write(&val).is_err() {
                                        return Ok(());
                                    }
                                }
                                x => {
                                    if stdin_write
                                        .write(x.into_string(", ", &config).as_bytes())
                                        .is_err()
                                    {
                                        return Err(());
                                    }
                                }
                            }
                        }
                        Ok(())
                    });
                }

                let last_expression = self.last_expression;
                let span = self.name.span;
                let output_ctrlc = ctrlc.clone();
                let (tx, rx) = mpsc::channel();

                std::thread::spawn(move || {
                    // If this external is not the last expression, then its output is piped to a channel
                    // and we create a ValueStream that can be consumed
                    if !last_expression {
                        let stdout = child.stdout.take().ok_or_else(|| {
                            ShellError::ExternalCommand(
                                "Error taking stdout from external".to_string(),
                                span,
                            )
                        })?;

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

                            if let Some(ctrlc) = &ctrlc {
                                if ctrlc.load(Ordering::SeqCst) {
                                    break;
                                }
                            }

                            match tx.send(data) {
                                Ok(_) => continue,
                                Err(_) => break,
                            }
                        }
                    }

                    match child.wait() {
                        Err(err) => Err(ShellError::ExternalCommand(format!("{}", err), span)),
                        Ok(_) => Ok(()),
                    }
                });
                // The ValueStream is consumed by the next expression in the pipeline
                let value =
                    ChannelReceiver::new(rx, self.name.span).into_pipeline_data(output_ctrlc);

                Ok(value)
            }
        }
    }

    fn create_command(&self) -> CommandSys {
        // in all the other cases shell out
        if cfg!(windows) {
            //TODO. This should be modifiable from the config file.
            // We could give the option to call from powershell
            // for minimal builds cwd is unused
            if self.name.item.ends_with(".cmd") || self.name.item.ends_with(".bat") {
                self.spawn_cmd_command()
            } else {
                self.spawn_simple_command()
            }
        } else if self.name.item.ends_with(".sh") {
            self.spawn_sh_command()
        } else {
            self.spawn_simple_command()
        }
    }

    /// Spawn a command without shelling out to an external shell
    fn spawn_simple_command(&self) -> std::process::Command {
        let mut process = std::process::Command::new(&self.name.item);

        for arg in &self.args {
            let arg = trim_enclosing_quotes(arg);
            let arg = nu_path::expand_path(arg).to_string_lossy().to_string();

            let arg = arg.replace("\\", "\\\\");

            process.arg(&arg);
        }

        process
    }

    /// Spawn a cmd command with `cmd /c args...`
    fn spawn_cmd_command(&self) -> std::process::Command {
        let mut process = std::process::Command::new("cmd");
        process.arg("/c");
        process.arg(&self.name.item);
        for arg in &self.args {
            // Clean the args before we use them:
            // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
            // cmd.exe needs to have a caret to escape a pipe
            let arg = arg.replace("|", "^|");
            process.arg(&arg);
        }
        process
    }

    /// Spawn a sh command with `sh -c args...`
    fn spawn_sh_command(&self) -> std::process::Command {
        let joined_and_escaped_arguments =
            self.args.iter().map(|arg| shell_arg_escape(arg)).join(" ");
        let cmd_with_args = vec![self.name.item.clone(), joined_and_escaped_arguments].join(" ");
        let mut process = std::process::Command::new("sh");
        process.arg("-c").arg(cmd_with_args);
        process
    }
}

fn has_unsafe_shell_characters(arg: &str) -> bool {
    let re: Regex = Regex::new(r"[^\w@%+=:,./-]").expect("regex to be valid");

    re.is_match(arg)
}

fn shell_arg_escape(arg: &str) -> String {
    match arg {
        "" => String::from("''"),
        s if !has_unsafe_shell_characters(s) => String::from(s),
        _ => {
            let single_quotes_escaped = arg.split('\'').join("'\"'\"'");
            format!("'{}'", single_quotes_escaped)
        }
    }
}

fn trim_enclosing_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('"'), Some('"')) => chars.collect(),
        (Some('\''), Some('\'')) => chars.collect(),
        _ => input.to_string(),
    }
}

// The piped data from stdout from the external command can be either String
// or binary. We use this enum to pass the data from the spawned process
#[derive(Debug)]
enum Data {
    String(String),
    Bytes(Vec<u8>),
}

// Receiver used for the ValueStream
// It implements iterator so it can be used as a ValueStream
struct ChannelReceiver {
    rx: mpsc::Receiver<Data>,
    span: Span,
}

impl ChannelReceiver {
    pub fn new(rx: mpsc::Receiver<Data>, span: Span) -> Self {
        Self { rx, span }
    }
}

impl Iterator for ChannelReceiver {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Ok(v) => match v {
                Data::String(s) => Some(Value::String {
                    val: s,
                    span: self.span,
                }),
                Data::Bytes(b) => Some(Value::Binary {
                    val: b,
                    span: self.span,
                }),
            },
            Err(_) => None,
        }
    }
}
