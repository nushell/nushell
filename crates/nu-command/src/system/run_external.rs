use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command as CommandSys, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use nu_engine::env_to_strings;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{ast::Call, engine::Command, ShellError, Signature, SyntaxShape, Value};
use nu_protocol::{Category, PipelineData, RawStream, Span, Spanned};

use itertools::Itertools;

use nu_engine::CallExt;
use pathdiff::diff_paths;
use regex::Regex;

const OUTPUT_BUFFER_SIZE: usize = 1024;

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
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let args: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let last_expression = call.has_flag("last_expression");

        // Translate environment variables from Values to Strings
        let config = stack.get_config().unwrap_or_default();
        let env_vars_str = env_to_strings(engine_state, stack, &config)?;

        let mut args_strs = vec![];

        for arg in args {
            let span = if let Ok(span) = arg.span() {
                span
            } else {
                Span { start: 0, end: 0 }
            };

            if let Ok(s) = arg.as_string() {
                args_strs.push(Spanned { item: s, span });
            } else if let Value::List { vals, span } = arg {
                // Interpret a list as a series of arguments
                for val in vals {
                    if let Ok(s) = val.as_string() {
                        args_strs.push(Spanned { item: s, span });
                    } else {
                        return Err(ShellError::ExternalCommand(
                            "Cannot convert argument to a string".into(),
                            "All arguments to an external command need to be string-compatible"
                                .into(),
                            val.span()?,
                        ));
                    }
                }
            } else {
                return Err(ShellError::ExternalCommand(
                    "Cannot convert argument to a string".into(),
                    "All arguments to an external command need to be string-compatible".into(),
                    arg.span()?,
                ));
            }
        }

        let command = ExternalCommand {
            name,
            args: args_strs,
            last_expression,
            env_vars: env_vars_str,
        };
        command.run_with_input(engine_state, stack, input)
    }
}

pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub last_expression: bool,
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

        let mut process = if let Some(d) = self.env_vars.get("PWD") {
            let mut process = self.create_command(d)?;
            process.current_dir(d);
            process
        } else {
            return Err(ShellError::SpannedLabeledErrorHelp(
                "Current directory not found".to_string(),
                "did not find PWD environment variable".to_string(),
                head,
                concat!(
                    "The environment variable 'PWD' was not found. ",
                    "It is required to define the current directory when running an external command."
                ).to_string(),
            ));
        };

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
                "can't run executable".to_string(),
                err.to_string(),
                self.name.span,
            )),
            Ok(mut child) => {
                if !input.is_nothing() {
                    let engine_state = engine_state.clone();
                    let mut stack = stack.clone();
                    stack.update_config(
                        "use_ansi_coloring",
                        Value::Bool {
                            val: false,
                            span: Span::new(0, 0),
                        },
                    );
                    // if there is a string or a stream, that is sent to the pipe std
                    if let Some(mut stdin_write) = child.stdin.take() {
                        std::thread::spawn(move || {
                            let input = crate::Table::run(
                                &crate::Table,
                                &engine_state,
                                &mut stack,
                                &Call::new(),
                                input,
                            );

                            if let Ok(input) = input {
                                for value in input.into_iter() {
                                    if let Value::String { val, span: _ } = value {
                                        if stdin_write.write(val.as_bytes()).is_err() {
                                            return Ok(());
                                        }
                                    } else {
                                        return Err(());
                                    }
                                }
                            }

                            Ok(())
                        });
                    }
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

                            match tx.send(bytes) {
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
                        Ok(_) => Ok(()),
                    }
                });
                let receiver = ChannelReceiver::new(rx);

                Ok(PipelineData::RawStream(
                    RawStream::new(Box::new(receiver), output_ctrlc, head),
                    head,
                    None,
                ))
            }
        }
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
        let head = if head.starts_with('~') || head.starts_with("..") {
            nu_path::expand_path_with(head, cwd)
                .to_string_lossy()
                .to_string()
        } else {
            head
        };

        //let head = head.replace("\\", "\\\\");

        let new_head;

        #[cfg(windows)]
        {
            new_head = head.replace("\\", "\\\\");
        }

        #[cfg(not(windows))]
        {
            new_head = head;
        }

        let mut process = std::process::Command::new(&new_head);

        for arg in self.args.iter() {
            let mut arg = Spanned {
                item: trim_enclosing_quotes(&arg.item),
                span: arg.span,
            };
            arg.item = if arg.item.starts_with('~') || arg.item.starts_with("..") {
                nu_path::expand_path_with(&arg.item, cwd)
                    .to_string_lossy()
                    .to_string()
            } else {
                arg.item
            };

            let cwd = PathBuf::from(cwd);

            if arg.item.contains('*') {
                if let Ok((prefix, matches)) = nu_engine::glob_from(&arg, &cwd, self.name.span) {
                    let matches: Vec<_> = matches.collect();

                    // Following shells like bash, if we can't expand a glob pattern, we don't assume an empty arg
                    // Instead, we throw an error. This helps prevent issues with things like `ls unknowndir/*` accidentally
                    // listening the current directory.
                    if matches.is_empty() {
                        return Err(ShellError::FileNotFoundCustom(
                            "pattern not found".to_string(),
                            arg.span,
                        ));
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
                            let new_arg;

                            #[cfg(windows)]
                            {
                                new_arg = arg.replace("\\", "\\\\");
                            }

                            #[cfg(not(windows))]
                            {
                                new_arg = arg;
                            }

                            process.arg(&new_arg);
                        } else {
                            let new_arg;

                            #[cfg(windows)]
                            {
                                new_arg = arg.item.replace("\\", "\\\\");
                            }

                            #[cfg(not(windows))]
                            {
                                new_arg = arg.item.clone();
                            }

                            process.arg(&new_arg);
                        }
                    }
                }
            } else {
                let new_arg;

                #[cfg(windows)]
                {
                    new_arg = arg.item.replace("\\", "\\\\");
                }

                #[cfg(not(windows))]
                {
                    new_arg = arg.item;
                }

                process.arg(&new_arg);
            }
        }

        Ok(process)
    }

    /// Spawn a cmd command with `cmd /c args...`
    pub fn spawn_cmd_command(&self) -> std::process::Command {
        let mut process = std::process::Command::new("cmd");
        process.arg("/c");
        process.arg(&self.name.item);
        for arg in &self.args {
            // Clean the args before we use them:
            // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
            // cmd.exe needs to have a caret to escape a pipe
            let arg = arg.item.replace("|", "^|");
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
        _ => input.to_string(),
    }
}

// Receiver used for the ValueStream
// It implements iterator so it can be used as a ValueStream
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
