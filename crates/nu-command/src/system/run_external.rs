use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as CommandSys, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use nu_engine::env_to_strings;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{ast::Call, engine::Command, ShellError, Signature, SyntaxShape, Value};
use nu_protocol::{Category, Example, ListStream, PipelineData, RawStream, Span, Spanned};

use itertools::Itertools;

use nu_engine::CallExt;
use pathdiff::diff_paths;
use regex::Regex;

const OUTPUT_BUFFER_SIZE: usize = 1024;
const OUTPUT_BUFFERS_IN_FLIGHT: usize = 3;

#[derive(Clone)]
pub struct External;

impl Command for External {
    fn name(&self) -> &str {
        "run-external"
    }

    fn usage(&self) -> &str {
        "Runs external command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .switch("redirect-stdout", "redirect-stdout", None)
            .switch("redirect-stderr", "redirect-stderr", None)
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
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let args: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let redirect_stdout = call.has_flag("redirect-stdout");
        let redirect_stderr = call.has_flag("redirect-stderr");

        // Translate environment variables from Values to Strings
        let env_vars_str = env_to_strings(engine_state, stack)?;

        fn value_as_spanned(value: Value) -> Result<Spanned<String>, ShellError> {
            let span = value.span()?;

            value
                .as_string()
                .map(|item| Spanned { item, span })
                .map_err(|_| {
                    ShellError::ExternalCommand(
                        "Cannot convert argument to a string".into(),
                        "All arguments to an external command need to be string-compatible".into(),
                        span,
                    )
                })
        }

        let mut spanned_args = vec![];
        for one_arg in args {
            match one_arg {
                Value::List { vals, .. } => {
                    // turn all the strings in the array into params.
                    // Example: one_arg may be something like ["ls" "-a"]
                    // convert it to "ls" "-a"
                    for v in vals {
                        spanned_args.push(value_as_spanned(v)?)
                    }
                }
                val => spanned_args.push(value_as_spanned(val)?),
            }
        }

        let command = ExternalCommand {
            name,
            args: spanned_args,
            redirect_stdout,
            redirect_stderr,
            env_vars: env_vars_str,
        };
        command.run_with_input(engine_state, stack, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Run an external command",
            example: r#"run-external "echo" "-n" "hello""#,
            result: None,
        }]
    }
}

pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub redirect_stdout: bool,
    pub redirect_stderr: bool,
    pub env_vars: HashMap<String, String>,
}

impl ExternalCommand {
    pub fn run_with_input(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = self.name.span;

        let ctrlc = engine_state.ctrlc.clone();

        let mut process = self.create_process(&input, false, head)?;
        let child;

        #[cfg(windows)]
        {
            match process.spawn() {
                Err(_) => {
                    let mut process = self.create_process(&input, true, head)?;
                    child = process.spawn();
                }
                Ok(process) => {
                    child = Ok(process);
                }
            }
        }

        #[cfg(not(windows))]
        {
            child = process.spawn()
        }

        match child {
            Err(err) => Err(ShellError::ExternalCommand(
                "can't run executable".to_string(),
                err.to_string(),
                self.name.span,
            )),
            Ok(mut child) => {
                if !input.is_nothing() {
                    let mut engine_state = engine_state.clone();
                    let mut stack = stack.clone();

                    // Turn off color as we pass data through
                    engine_state.config.use_ansi_coloring = false;

                    // if there is a string or a stream, that is sent to the pipe std
                    if let Some(mut stdin_write) = child.stdin.take() {
                        std::thread::spawn(move || {
                            let input = crate::Table::run(
                                &crate::Table,
                                &engine_state,
                                &mut stack,
                                &Call::new(head),
                                input,
                            );

                            if let Ok(input) = input {
                                for value in input.into_iter() {
                                    let buf = match value {
                                        Value::String { val, .. } => val.into_bytes(),
                                        Value::Binary { val, .. } => val,
                                        _ => return Err(()),
                                    };
                                    if stdin_write.write(&buf).is_err() {
                                        return Ok(());
                                    }
                                }
                            }

                            Ok(())
                        });
                    }
                }

                let redirect_stdout = self.redirect_stdout;
                let redirect_stderr = self.redirect_stderr;
                let span = self.name.span;
                let output_ctrlc = ctrlc.clone();
                let (stdout_tx, stdout_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                let (stderr_tx, stderr_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                let (exit_code_tx, exit_code_rx) = mpsc::channel();

                std::thread::spawn(move || {
                    // If this external is not the last expression, then its output is piped to a channel
                    // and we create a ListStream that can be consumed

                    if redirect_stderr {
                        let stderr = child.stderr.take().ok_or_else(|| {
                            ShellError::ExternalCommand(
                                "Error taking stderr from external".to_string(),
                                "Redirects need access to stderr of an external command"
                                    .to_string(),
                                span,
                            )
                        })?;

                        // Stderr is read using the Buffer reader. It will do so until there is an
                        // error or there are no more bytes to read
                        let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, stderr);
                        while let Ok(bytes) = buf_read.fill_buf() {
                            if bytes.is_empty() {
                                break;
                            }

                            // The Cow generated from the function represents the conversion
                            // from bytes to String. If no replacements are required, then the
                            // borrowed value is a proper UTF-8 string. The Owned option represents
                            // a string where the values had to be replaced, thus marking it as bytes
                            let bytes = bytes.to_vec();
                            let length = bytes.len();
                            buf_read.consume(length);

                            if let Some(ctrlc) = &ctrlc {
                                if ctrlc.load(Ordering::SeqCst) {
                                    break;
                                }
                            }

                            match stderr_tx.send(bytes) {
                                Ok(_) => continue,
                                Err(_) => break,
                            }
                        }
                    }

                    if redirect_stdout {
                        let stdout = child.stdout.take().ok_or_else(|| {
                            ShellError::ExternalCommand(
                                "Error taking stdout from external".to_string(),
                                "Redirects need access to stdout of an external command"
                                    .to_string(),
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
                            let bytes = bytes.to_vec();
                            let length = bytes.len();
                            buf_read.consume(length);

                            if let Some(ctrlc) = &ctrlc {
                                if ctrlc.load(Ordering::SeqCst) {
                                    break;
                                }
                            }

                            match stdout_tx.send(bytes) {
                                Ok(_) => continue,
                                Err(_) => break,
                            }
                        }
                    }

                    match child.wait() {
                        Err(err) => Err(ShellError::ExternalCommand(
                            "External command exited with error".into(),
                            err.to_string(),
                            span,
                        )),
                        Ok(x) => {
                            if let Some(code) = x.code() {
                                let _ = exit_code_tx.send(Value::Int {
                                    val: code as i64,
                                    span: head,
                                });
                            } else if x.success() {
                                let _ = exit_code_tx.send(Value::Int { val: 0, span: head });
                            } else {
                                let _ = exit_code_tx.send(Value::Int {
                                    val: -1,
                                    span: head,
                                });
                            }
                            Ok(())
                        }
                    }
                });
                let stdout_receiver = ChannelReceiver::new(stdout_rx);
                let stderr_receiver = ChannelReceiver::new(stderr_rx);
                let exit_code_receiver = ValueReceiver::new(exit_code_rx);

                Ok(PipelineData::ExternalStream {
                    stdout: if redirect_stdout {
                        Some(RawStream::new(
                            Box::new(stdout_receiver),
                            output_ctrlc.clone(),
                            head,
                        ))
                    } else {
                        None
                    },
                    stderr: Some(RawStream::new(
                        Box::new(stderr_receiver),
                        output_ctrlc.clone(),
                        head,
                    )),
                    exit_code: Some(ListStream::from_stream(
                        Box::new(exit_code_receiver),
                        output_ctrlc,
                    )),
                    span: head,
                    metadata: None,
                })
            }
        }
    }

    fn create_process(
        &self,
        input: &PipelineData,
        use_cmd: bool,
        span: Span,
    ) -> Result<CommandSys, ShellError> {
        let mut process = if let Some(d) = self.env_vars.get("PWD") {
            let mut process = if use_cmd {
                self.spawn_cmd_command()
            } else {
                self.create_command(d)?
            };

            // do not try to set current directory if cwd does not exist
            if Path::new(&d).exists() {
                process.current_dir(d);
            }
            process
        } else {
            return Err(ShellError::GenericError(
                "Current directory not found".to_string(),
                "did not find PWD environment variable".to_string(),
                Some(span),
                Some(concat!(
                    "The environment variable 'PWD' was not found. ",
                    "It is required to define the current directory when running an external command."
                ).to_string()),
                Vec::new(),
            ));
        };

        process.envs(&self.env_vars);

        // If the external is not the last command, its output will get piped
        // either as a string or binary
        if self.redirect_stdout {
            process.stdout(Stdio::piped());
        }

        if self.redirect_stderr {
            process.stderr(Stdio::piped());
        }

        // If there is an input from the pipeline. The stdin from the process
        // is piped so it can be used to send the input information
        if !matches!(input, PipelineData::Value(Value::Nothing { .. }, ..)) {
            process.stdin(Stdio::piped());
        }

        Ok(process)
    }

    fn create_command(&self, cwd: &str) -> Result<CommandSys, ShellError> {
        // in all the other cases shell out
        if cfg!(windows) {
            //TODO. This should be modifiable from the config file.
            // We could give the option to call from powershell
            // for minimal builds cwd is unused
            if self.name.item.ends_with(".cmd") || self.name.item.ends_with(".bat") {
                Ok(self.spawn_cmd_command())
            } else {
                self.spawn_simple_command(cwd)
            }
        } else if self.name.item.ends_with(".sh") {
            Ok(self.spawn_sh_command())
        } else {
            self.spawn_simple_command(cwd)
        }
    }

    /// Spawn a command without shelling out to an external shell
    pub fn spawn_simple_command(&self, cwd: &str) -> Result<std::process::Command, ShellError> {
        let head = trim_enclosing_quotes(&self.name.item);
        let head = nu_path::expand_to_real_path(head)
            .to_string_lossy()
            .to_string();

        let mut process = std::process::Command::new(&head);

        for arg in self.args.iter() {
            let mut arg = Spanned {
                item: trim_enclosing_quotes(&arg.item),
                span: arg.span,
            };
            arg.item = nu_path::expand_to_real_path(arg.item)
                .to_string_lossy()
                .to_string();

            let cwd = PathBuf::from(cwd);

            if arg.item.contains('*') {
                if let Ok((prefix, matches)) =
                    nu_engine::glob_from(&arg, &cwd, self.name.span, None)
                {
                    let matches: Vec<_> = matches.collect();

                    // FIXME: do we want to special-case this further? We might accidentally expand when they don't
                    // intend to
                    if matches.is_empty() {
                        process.arg(&arg.item);
                    }
                    for m in matches {
                        if let Ok(arg) = m {
                            let arg = if let Some(prefix) = &prefix {
                                if let Ok(remainder) = arg.strip_prefix(&prefix) {
                                    let new_prefix = if let Some(pfx) = diff_paths(&prefix, &cwd) {
                                        pfx
                                    } else {
                                        prefix.to_path_buf()
                                    };

                                    new_prefix.join(remainder).to_string_lossy().to_string()
                                } else {
                                    arg.to_string_lossy().to_string()
                                }
                            } else {
                                arg.to_string_lossy().to_string()
                            };

                            process.arg(&arg);
                        } else {
                            process.arg(&arg.item);
                        }
                    }
                }
            } else {
                process.arg(&arg.item);
            }
        }

        Ok(process)
    }

    /// Spawn a cmd command with `cmd /c args...`
    pub fn spawn_cmd_command(&self) -> std::process::Command {
        let mut process = std::process::Command::new("cmd");

        // Disable AutoRun
        // TODO: There should be a config option to enable/disable this
        // Alternatively (even better) a config option to specify all the arguments to pass to cmd
        process.arg("/D");

        process.arg("/c");
        process.arg(&self.name.item);
        for arg in &self.args {
            // Clean the args before we use them:
            // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
            // cmd.exe needs to have a caret to escape a pipe
            let arg = arg.item.replace('|', "^|");
            process.arg(&arg);
        }
        process
    }

    /// Spawn a sh command with `sh -c args...`
    pub fn spawn_sh_command(&self) -> std::process::Command {
        let joined_and_escaped_arguments = self
            .args
            .iter()
            .map(|arg| shell_arg_escape(&arg.item))
            .join(" ");
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
        (Some('`'), Some('`')) => chars.collect(),
        _ => input.to_string(),
    }
}

// Receiver used for the RawStream
// It implements iterator so it can be used as a RawStream
struct ChannelReceiver {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl ChannelReceiver {
    pub fn new(rx: mpsc::Receiver<Vec<u8>>) -> Self {
        Self { rx }
    }
}

impl Iterator for ChannelReceiver {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Ok(v) => Some(Ok(v)),
            Err(_) => None,
        }
    }
}

// Receiver used for the ListStream
// It implements iterator so it can be used as a ListStream
struct ValueReceiver {
    rx: mpsc::Receiver<Value>,
}

impl ValueReceiver {
    pub fn new(rx: mpsc::Receiver<Value>) -> Self {
        Self { rx }
    }
}

impl Iterator for ValueReceiver {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}
