use fancy_regex::Regex;
use itertools::Itertools;
use nu_engine::env_to_strings;
use nu_engine::CallExt;
use nu_protocol::ast::{Expr, Expression};
use nu_protocol::did_you_mean;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{ast::Call, engine::Command, ShellError, Signature, SyntaxShape, Value};
use nu_protocol::{Category, Example, ListStream, PipelineData, RawStream, Span, Spanned};
use nu_system::ForegroundProcess;
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as CommandSys, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::Arc;

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
            .switch("redirect-stdout", "redirect stdout to the pipeline", None)
            .switch("redirect-stderr", "redirect stderr to the pipeline", None)
            .required("command", SyntaxShape::Any, "external command to run")
            .rest("args", SyntaxShape::Any, "arguments for external command")
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
        let args_expr: Vec<Expression> = call.positional_iter().skip(1).cloned().collect();
        let mut arg_keep_raw = vec![];
        for (one_arg, one_arg_expr) in args.into_iter().zip(args_expr) {
            match one_arg {
                Value::List { vals, .. } => {
                    // turn all the strings in the array into params.
                    // Example: one_arg may be something like ["ls" "-a"]
                    // convert it to "ls" "-a"
                    for v in vals {
                        spanned_args.push(value_as_spanned(v)?);
                        // for arguments in list, it's always treated as a whole arguments
                        arg_keep_raw.push(true);
                    }
                }
                val => {
                    spanned_args.push(value_as_spanned(val)?);
                    match one_arg_expr.expr {
                        // refer to `parse_dollar_expr` function
                        // the expression type of $variable_name, $"($variable_name)"
                        // will be Expr::StringInterpolation, Expr::FullCellPath
                        Expr::StringInterpolation(_) | Expr::FullCellPath(_) => {
                            arg_keep_raw.push(true)
                        }
                        _ => arg_keep_raw.push(false),
                    }
                    {}
                }
            }
        }

        let command = ExternalCommand {
            name,
            args: spanned_args,
            arg_keep_raw,
            redirect_stdout,
            redirect_stderr,
            env_vars: env_vars_str,
        };
        command.run_with_input(engine_state, stack, input, false)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run an external command",
                example: r#"run-external "echo" "-n" "hello""#,
                result: None,
            },
            Example {
                description: "Redirect stdout from an external command into the pipeline",
                example: r#"run-external --redirect-stdout "echo" "-n" "hello" | split chars"#,
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub arg_keep_raw: Vec<bool>,
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
        reconfirm_command_name: bool,
    ) -> Result<PipelineData, ShellError> {
        let head = self.name.span;

        let ctrlc = engine_state.ctrlc.clone();

        let mut fg_process = ForegroundProcess::new(
            self.create_process(&input, false, head)?,
            engine_state.pipeline_externals_state.clone(),
        );
        // mut is used in the windows branch only, suppress warning on other platforms
        #[allow(unused_mut)]
        let mut child;

        #[cfg(windows)]
        {
            // Running external commands on Windows has 2 points of complication:
            // 1. Some common Windows commands are actually built in to cmd.exe, not executables in their own right.
            // 2. We need to let users run batch scripts etc. (.bat, .cmd) without typing their extension

            // To support these situations, we have a fallback path that gets run if a command
            // fails to be run as a normal executable:
            // 1. "shell out" to cmd.exe if the command is a known cmd.exe internal command
            // 2. Otherwise, use `which-rs` to look for batch files etc. then run those in cmd.exe
            match fg_process.spawn() {
                Err(err) => {
                    // set the default value, maybe we'll override it later
                    child = Err(err);

                    // This has the full list of cmd.exe "internal" commands: https://ss64.com/nt/syntax-internal.html
                    // I (Reilly) went through the full list and whittled it down to ones that are potentially useful:
                    const CMD_INTERNAL_COMMANDS: [&str; 10] = [
                        "ASSOC", "CLS", "DIR", "ECHO", "FTYPE", "MKLINK", "PAUSE", "START", "VER",
                        "VOL",
                    ];
                    let command_name_upper = self.name.item.to_uppercase();
                    let looks_like_cmd_internal = CMD_INTERNAL_COMMANDS
                        .iter()
                        .any(|&cmd| command_name_upper == cmd);

                    if looks_like_cmd_internal {
                        let mut cmd_process = ForegroundProcess::new(
                            self.create_process(&input, true, head)?,
                            engine_state.pipeline_externals_state.clone(),
                        );
                        child = cmd_process.spawn();
                    } else {
                        #[cfg(feature = "which-support")]
                        {
                            // maybe it's a batch file (foo.cmd) and the user typed `foo`. Try to find it with `which-rs`
                            // TODO: clean this up with an if-let chain once those are stable
                            if let Ok(path) =
                                nu_engine::env::path_str(engine_state, stack, self.name.span)
                            {
                                if let Some(cwd) = self.env_vars.get("PWD") {
                                    // append cwd to PATH so `which-rs` looks in the cwd too.
                                    // this approximates what cmd.exe does.
                                    let path_with_cwd = format!("{};{}", cwd, path);
                                    if let Ok(which_path) =
                                        which::which_in(&self.name.item, Some(path_with_cwd), cwd)
                                    {
                                        if let Some(file_name) = which_path.file_name() {
                                            let file_name_upper =
                                                file_name.to_string_lossy().to_uppercase();
                                            if file_name_upper != command_name_upper {
                                                // which-rs found an executable file with a slightly different name
                                                // than the one the user tried. Let's try running it
                                                let mut new_command = self.clone();
                                                new_command.name = Spanned {
                                                    item: file_name.to_string_lossy().to_string(),
                                                    span: self.name.span,
                                                };
                                                let mut cmd_process = ForegroundProcess::new(
                                                    new_command
                                                        .create_process(&input, true, head)?,
                                                    engine_state.pipeline_externals_state.clone(),
                                                );
                                                child = cmd_process.spawn();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(process) => {
                    child = Ok(process);
                }
            }
        }

        #[cfg(not(windows))]
        {
            child = fg_process.spawn()
        }

        match child {
            Err(err) => {
                match err.kind() {
                    // If file not found, try suggesting alternative commands to the user
                    std::io::ErrorKind::NotFound => {
                        // recommend a replacement if the user tried a deprecated command
                        let command_name_lower = self.name.item.to_lowercase();
                        let deprecated = crate::deprecated_commands();
                        if deprecated.contains_key(&command_name_lower) {
                            let replacement = match deprecated.get(&command_name_lower) {
                                Some(s) => s.clone(),
                                None => "".to_string(),
                            };
                            return Err(ShellError::DeprecatedCommand(
                                command_name_lower,
                                replacement,
                                self.name.span,
                            ));
                        }

                        let suggestion = suggest_command(&self.name.item, engine_state);
                        let label = match suggestion {
                            Some(s) => {
                                if reconfirm_command_name {
                                    format!(
                                        "'{}' was not found, did you mean '{s}'?",
                                        self.name.item
                                    )
                                } else if self.name.item == s {
                                    let sugg = engine_state.which_module_has_decl(s.as_bytes());
                                    if let Some(sugg) = sugg {
                                        let sugg = String::from_utf8_lossy(sugg);
                                        format!("command '{s}' was not found but it exists in module '{sugg}'; try using `{sugg} {s}`")
                                    } else {
                                        format!("did you mean '{s}'?")
                                    }
                                } else {
                                    format!("did you mean '{s}'?")
                                }
                            }
                            None => {
                                if reconfirm_command_name {
                                    format!("executable '{}' was not found", self.name.item)
                                } else {
                                    "executable was not found".into()
                                }
                            }
                        };

                        Err(ShellError::ExternalCommand(
                            label,
                            err.to_string(),
                            self.name.span,
                        ))
                    }
                    // otherwise, a default error message
                    _ => Err(ShellError::ExternalCommand(
                        "can't run executable".into(),
                        err.to_string(),
                        self.name.span,
                    )),
                }
            }
            Ok(mut child) => {
                if !input.is_nothing() {
                    let mut engine_state = engine_state.clone();
                    let mut stack = stack.clone();

                    // Turn off color as we pass data through
                    engine_state.config.use_ansi_coloring = false;

                    // if there is a string or a stream, that is sent to the pipe std
                    if let Some(mut stdin_write) = child.as_mut().stdin.take() {
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

                #[cfg(unix)]
                let commandname = self.name.item.clone();
                let redirect_stdout = self.redirect_stdout;
                let redirect_stderr = self.redirect_stderr;
                let span = self.name.span;
                let output_ctrlc = ctrlc.clone();
                let stderr_ctrlc = ctrlc.clone();
                let (stdout_tx, stdout_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                let (exit_code_tx, exit_code_rx) = mpsc::channel();

                let stdout = child.as_mut().stdout.take();
                let stderr = child.as_mut().stderr.take();
                // If this external is not the last expression, then its output is piped to a channel
                // and we create a ListStream that can be consumed
                //
                // Create two threads: one for redirect stdout message, and wait for child process to complete.
                // The other may be created when we want to redirect stderr message.
                std::thread::spawn(move || {
                    if redirect_stdout {
                        let stdout = stdout.ok_or_else(|| {
                            ShellError::ExternalCommand(
                                "Error taking stdout from external".to_string(),
                                "Redirects need access to stdout of an external command"
                                    .to_string(),
                                span,
                            )
                        })?;

                        read_and_redirect_message(stdout, stdout_tx, ctrlc)
                    }

                    match child.as_mut().wait() {
                        Err(err) => Err(ShellError::ExternalCommand(
                            "External command exited with error".into(),
                            err.to_string(),
                            span,
                        )),
                        Ok(x) => {
                            #[cfg(unix)]
                            {
                                use nu_ansi_term::{Color, Style};
                                use std::os::unix::process::ExitStatusExt;
                                if x.core_dumped() {
                                    let style = Style::new().bold().on(Color::Red);
                                    println!(
                                        "{}",
                                        style.paint(format!(
                                            "nushell: oops, process '{commandname}' core dumped"
                                        ))
                                    );
                                    let _ = exit_code_tx.send(Value::Error {
                                        error: ShellError::ExternalCommand(
                                            "core dumped".to_string(),
                                            format!("Child process '{commandname}' core dumped"),
                                            head,
                                        ),
                                    });
                                    return Ok(());
                                }
                            }
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

                let (stderr_tx, stderr_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                if redirect_stderr {
                    std::thread::spawn(move || {
                        let stderr = stderr.ok_or_else(|| {
                            ShellError::ExternalCommand(
                                "Error taking stderr from external".to_string(),
                                "Redirects need access to stderr of an external command"
                                    .to_string(),
                                span,
                            )
                        })?;

                        read_and_redirect_message(stderr, stderr_tx, stderr_ctrlc);
                        Ok::<(), ShellError>(())
                    });
                }

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
                    stderr: if redirect_stderr {
                        Some(RawStream::new(
                            Box::new(stderr_receiver),
                            output_ctrlc.clone(),
                            head,
                        ))
                    } else {
                        None
                    },
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
        let (head, _, _) = trim_enclosing_quotes(&self.name.item);
        let head = nu_path::expand_to_real_path(head)
            .to_string_lossy()
            .to_string();

        let mut process = std::process::Command::new(head);

        for (arg, arg_keep_raw) in self.args.iter().zip(self.arg_keep_raw.iter()) {
            // if arg is quoted, like "aa", 'aa', `aa`, or:
            // if arg is a variable or String interpolation, like: $variable_name, $"($variable_name)"
            // `as_a_whole` will be true, so nu won't remove the inner quotes.
            let (trimmed_args, run_glob_expansion, mut keep_raw) = trim_enclosing_quotes(&arg.item);
            if *arg_keep_raw {
                keep_raw = true;
            }

            let mut arg = Spanned {
                item: if keep_raw {
                    trimmed_args
                } else {
                    remove_quotes(trimmed_args)
                },
                span: arg.span,
            };

            arg.item = nu_path::expand_tilde(arg.item)
                .to_string_lossy()
                .to_string();

            let cwd = PathBuf::from(cwd);

            if arg.item.contains('*') && run_glob_expansion {
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

/// Given an invalid command name, try to suggest an alternative
fn suggest_command(attempted_command: &str, engine_state: &EngineState) -> Option<String> {
    let commands = engine_state.get_signatures(false);
    let command_name_lower = attempted_command.to_lowercase();
    let search_term_match = commands.iter().find(|sig| {
        sig.search_terms
            .iter()
            .any(|term| term.to_lowercase() == command_name_lower)
    });
    match search_term_match {
        Some(sig) => Some(sig.name.clone()),
        None => {
            let command_names: Vec<String> = commands.iter().map(|sig| sig.name.clone()).collect();
            did_you_mean(&command_names, attempted_command)
        }
    }
}

fn has_unsafe_shell_characters(arg: &str) -> bool {
    let re: Regex = Regex::new(r"[^\w@%+=:,./-]").expect("regex to be valid");

    re.is_match(arg).unwrap_or(false)
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

/// This function returns a tuple with 3 items:
/// 1st item: trimmed string.
/// 2nd item: a boolean value indicate if it's ok to run glob expansion.
/// 3rd item: a boolean value indicate if we need to keep raw string.
fn trim_enclosing_quotes(input: &str) -> (String, bool, bool) {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('"'), Some('"')) => (chars.collect(), false, true),
        (Some('\''), Some('\'')) => (chars.collect(), false, true),
        (Some('`'), Some('`')) => (chars.collect(), true, true),
        _ => (input.to_string(), true, false),
    }
}

fn remove_quotes(input: String) -> String {
    let mut chars = input.chars();

    match (chars.next_back(), input.contains('=')) {
        (Some('"'), true) => chars
            .collect::<String>()
            .replacen('"', "", 1)
            .replace(r#"\""#, "\""),
        (Some('\''), true) => chars.collect::<String>().replacen('\'', "", 1),
        _ => input,
    }
}

// read message from given `reader`, and send out through `sender`.
//
// `ctrlc` is used to control the process, if ctrl-c is pressed, the read and redirect
// process will be breaked.
fn read_and_redirect_message<R>(
    reader: R,
    sender: SyncSender<Vec<u8>>,
    ctrlc: Option<Arc<AtomicBool>>,
) where
    R: Read,
{
    // read using the BufferReader. It will do so until there is an
    // error or there are no more bytes to read
    let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);
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

        match sender.send(bytes) {
            Ok(_) => continue,
            Err(_) => break,
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn remove_quotes_argument_with_equal_test() {
        let input = r#"--file="my_file.txt""#.into();
        let res = remove_quotes(input);

        assert_eq!("--file=my_file.txt", res)
    }

    #[test]
    fn argument_without_equal_test() {
        let input = r#"--file "my_file.txt""#.into();
        let res = remove_quotes(input);

        assert_eq!(r#"--file "my_file.txt""#, res)
    }

    #[test]
    fn remove_quotes_argument_with_single_quotes_test() {
        let input = r#"--file='my_file.txt'"#.into();
        let res = remove_quotes(input);

        assert_eq!("--file=my_file.txt", res)
    }

    #[test]
    fn argument_with_inner_quotes_test() {
        let input = r#"bash -c 'echo a'"#.into();
        let res = remove_quotes(input);

        assert_eq!("bash -c 'echo a'", res)
    }
}
