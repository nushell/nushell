use crate::hook::eval_hook;
use nu_engine::env_to_strings;
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, Expr, Expression},
    did_you_mean,
    engine::{Command, EngineState, Stack},
    Category, Example, ListStream, PipelineData, RawStream, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use nu_system::ForegroundProcess;
use os_pipe::PipeReader;
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as CommandSys, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, SyncSender};
use std::sync::Arc;
use std::thread;

const OUTPUT_BUFFER_SIZE: usize = 1024;
const OUTPUT_BUFFERS_IN_FLIGHT: usize = 3;

#[derive(Clone)]
pub struct External;

impl Command for External {
    fn name(&self) -> &str {
        "run-external"
    }

    fn usage(&self) -> &str {
        "Runs external command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch("redirect-stdout", "redirect stdout to the pipeline", None)
            .switch("redirect-stderr", "redirect stderr to the pipeline", None)
            .switch(
                "redirect-combine",
                "redirect both stdout and stderr combined to the pipeline (collected in stdout)",
                None,
            )
            .switch("trim-end-newline", "trimming end newlines", None)
            .required("command", SyntaxShape::String, "external command to run")
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
        let redirect_stdout = call.has_flag("redirect-stdout");
        let redirect_stderr = call.has_flag("redirect-stderr");
        let redirect_combine = call.has_flag("redirect-combine");
        let trim_end_newline = call.has_flag("trim-end-newline");

        if redirect_combine && (redirect_stdout || redirect_stderr) {
            return Err(ShellError::ExternalCommand {
                label: "Cannot use --redirect-combine with --redirect-stdout or --redirect-stderr"
                    .into(),
                help: "use either --redirect-combine or redirect a single output stream".into(),
                span: call.head,
            });
        }

        let command = create_external_command(
            engine_state,
            stack,
            call,
            redirect_stdout,
            redirect_stderr,
            redirect_combine,
            trim_end_newline,
        )?;

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

/// Creates ExternalCommand from a call
pub fn create_external_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    redirect_stdout: bool,
    redirect_stderr: bool,
    redirect_combine: bool,
    trim_end_newline: bool,
) -> Result<ExternalCommand, ShellError> {
    let name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let args: Vec<Value> = call.rest(engine_state, stack, 1)?;

    // Translate environment variables from Values to Strings
    let env_vars_str = env_to_strings(engine_state, stack)?;

    fn value_as_spanned(value: Value) -> Result<Spanned<String>, ShellError> {
        let span = value.span()?;

        value
            .as_string()
            .map(|item| Spanned { item, span })
            .map_err(|_| ShellError::ExternalCommand {
                label: format!("Cannot convert {} to a string", value.get_type()),
                help: "All arguments to an external command need to be string-compatible".into(),
                span,
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
                    Expr::StringInterpolation(_) | Expr::FullCellPath(_) => arg_keep_raw.push(true),
                    _ => arg_keep_raw.push(false),
                }
                {}
            }
        }
    }

    Ok(ExternalCommand {
        name,
        args: spanned_args,
        arg_keep_raw,
        redirect_stdout,
        redirect_stderr,
        redirect_combine,
        env_vars: env_vars_str,
        trim_end_newline,
    })
}

#[derive(Clone)]
pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub arg_keep_raw: Vec<bool>,
    pub redirect_stdout: bool,
    pub redirect_stderr: bool,
    pub redirect_combine: bool,
    pub env_vars: HashMap<String, String>,
    pub trim_end_newline: bool,
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

        #[allow(unused_mut)]
        let (cmd, mut reader) = self.create_process(&input, false, head)?;
        let mut fg_process =
            ForegroundProcess::new(cmd, engine_state.pipeline_externals_state.clone());
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
                    const CMD_INTERNAL_COMMANDS: [&str; 9] = [
                        "ASSOC", "CLS", "ECHO", "FTYPE", "MKLINK", "PAUSE", "START", "VER", "VOL",
                    ];
                    let command_name_upper = self.name.item.to_uppercase();
                    let looks_like_cmd_internal = CMD_INTERNAL_COMMANDS
                        .iter()
                        .any(|&cmd| command_name_upper == cmd);

                    if looks_like_cmd_internal {
                        let (cmd, new_reader) = self.create_process(&input, true, head)?;
                        reader = new_reader;
                        let mut cmd_process = ForegroundProcess::new(
                            cmd,
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
                                                let (cmd, new_reader) = new_command
                                                    .create_process(&input, true, head)?;
                                                reader = new_reader;
                                                let mut cmd_process = ForegroundProcess::new(
                                                    cmd,
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
                                        "'{}' was not found; did you mean '{s}'?",
                                        self.name.item
                                    )
                                } else {
                                    let cmd_name = &self.name.item;
                                    let maybe_module = engine_state
                                        .which_module_has_decl(cmd_name.as_bytes(), &[]);
                                    if let Some(module_name) = maybe_module {
                                        let module_name = String::from_utf8_lossy(module_name);
                                        let new_name = &[module_name.as_ref(), cmd_name].join(" ");

                                        if engine_state
                                            .find_decl(new_name.as_bytes(), &[])
                                            .is_some()
                                        {
                                            format!("command '{cmd_name}' was not found but it was imported from module '{module_name}'; try using `{new_name}`")
                                        } else {
                                            format!("command '{cmd_name}' was not found but it exists in module '{module_name}'; try importing it with `use`")
                                        }
                                    } else {
                                        // If command and suggestion are the same, display not found
                                        if cmd_name == &s {
                                            format!("'{cmd_name}' was not found")
                                        } else {
                                            format!("did you mean '{s}'?")
                                        }
                                    }
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

                        let mut err_str = err.to_string();
                        if engine_state.is_interactive {
                            let mut engine_state = engine_state.clone();
                            if let Some(hook) = engine_state.config.hooks.command_not_found.clone()
                            {
                                if let Ok(PipelineData::Value(Value::String { val, .. }, ..)) =
                                    eval_hook(
                                        &mut engine_state,
                                        stack,
                                        None,
                                        vec![(
                                            "cmd_name".into(),
                                            Value::string(
                                                self.name.item.to_string(),
                                                self.name.span,
                                            ),
                                        )],
                                        &hook,
                                    )
                                {
                                    err_str = format!("{}\n{}", err_str, val);
                                }
                            }
                        }

                        Err(ShellError::ExternalCommand {
                            label,
                            help: err_str,
                            span: self.name.span,
                        })
                    }
                    // otherwise, a default error message
                    _ => Err(ShellError::ExternalCommand {
                        label: "can't run executable".into(),
                        help: err.to_string(),
                        span: self.name.span,
                    }),
                }
            }
            Ok(mut child) => {
                if !input.is_nothing() {
                    let mut engine_state = engine_state.clone();
                    let mut stack = stack.clone();

                    // Turn off color as we pass data through
                    engine_state.config.use_ansi_coloring = false;

                    // Pipe input into the external command's stdin
                    if let Some(mut stdin_write) = child.as_mut().stdin.take() {
                        thread::Builder::new()
                            .name("external stdin worker".to_string())
                            .spawn(move || {
                                // Attempt to render the input as a table before piping it to the external.
                                // This is important for pagers like `less`;
                                // they need to get Nu data rendered for display to users.
                                //
                                // TODO: should we do something different for list<string> inputs?
                                // Users often expect those to be piped to *nix tools as raw strings separated by newlines
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
                            })
                            .expect("Failed to create thread");
                    }
                }

                #[cfg(unix)]
                let commandname = self.name.item.clone();
                let redirect_stdout = self.redirect_stdout;
                let redirect_stderr = self.redirect_stderr;
                let redirect_combine = self.redirect_combine;
                let span = self.name.span;
                let output_ctrlc = ctrlc.clone();
                let stderr_ctrlc = ctrlc.clone();
                let (stdout_tx, stdout_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                let (exit_code_tx, exit_code_rx) = mpsc::channel();

                let stdout = child.as_mut().stdout.take();
                let stderr = child.as_mut().stderr.take();

                // If this external is not the last expression, then its output is piped to a channel
                // and we create a ListStream that can be consumed

                // First create a thread to redirect the external's stdout and wait for an exit code.
                thread::Builder::new()
                    .name("stdout redirector + exit code waiter".to_string())
                    .spawn(move || {
                        if redirect_stdout {
                            let stdout = stdout.ok_or_else(|| {
                                ShellError::ExternalCommand { label: "Error taking stdout from external".to_string(), help: "Redirects need access to stdout of an external command"
                                        .to_string(), span }
                            })?;

                            read_and_redirect_message(stdout, stdout_tx, ctrlc)
                        } else if redirect_combine {
                            let stdout = reader.ok_or_else(|| {
                                ShellError::ExternalCommand { label: "Error taking combined stdout and stderr from external".to_string(), help: "Combined redirects need access to reader pipe of an external command"
                                        .to_string(), span }
                            })?;
                            read_and_redirect_message(stdout, stdout_tx, ctrlc)
                        }

                    match child.as_mut().wait() {
                        Err(err) => Err(ShellError::ExternalCommand { label: "External command exited with error".into(), help: err.to_string(), span }),
                        Ok(x) => {
                            #[cfg(unix)]
                            {
                                use nu_ansi_term::{Color, Style};
                                use std::ffi::CStr;
                                use std::os::unix::process::ExitStatusExt;

                                if x.core_dumped() {
                                    let cause = x.signal().and_then(|sig| unsafe {
                                        // SAFETY: We should be the first to call `char * strsignal(int sig)`
                                        let sigstr_ptr = libc::strsignal(sig);
                                        if sigstr_ptr.is_null() {
                                            return None;
                                        }

                                        // SAFETY: The pointer points to a valid non-null string
                                        let sigstr = CStr::from_ptr(sigstr_ptr);
                                        sigstr.to_str().map(String::from).ok()
                                    });

                                    let cause = cause.as_deref().unwrap_or("Something went wrong");

                                    let style = Style::new().bold().on(Color::Red);
                                    eprintln!(
                                        "{}",
                                        style.paint(format!(
                                            "{cause}: oops, process '{commandname}' core dumped"
                                        ))
                                    );
                                    let _ = exit_code_tx.send(Value::Error {
                                        error: Box::new(ShellError::ExternalCommand { label: "core dumped".to_string(), help: format!("{cause}: child process '{commandname}' core dumped"), span: head }),
                                    });
                                    return Ok(());
                                }
                            }
                            if let Some(code) = x.code() {
                                let _ = exit_code_tx.send(Value::int(code as i64, head));
                            } else if x.success() {
                                let _ = exit_code_tx.send(Value::int(0, head));
                            } else {
                                let _ = exit_code_tx.send(Value::int(-1, head));
                            }
                            Ok(())
                        }
                    }
                }).expect("Failed to create thread");

                let (stderr_tx, stderr_rx) = mpsc::sync_channel(OUTPUT_BUFFERS_IN_FLIGHT);
                if redirect_stderr {
                    thread::Builder::new()
                        .name("stderr redirector".to_string())
                        .spawn(move || {
                            let stderr = stderr.ok_or_else(|| ShellError::ExternalCommand {
                                label: "Error taking stderr from external".to_string(),
                                help: "Redirects need access to stderr of an external command"
                                    .to_string(),
                                span,
                            })?;

                            read_and_redirect_message(stderr, stderr_tx, stderr_ctrlc);
                            Ok::<(), ShellError>(())
                        })
                        .expect("Failed to create thread");
                }

                let stdout_receiver = ChannelReceiver::new(stdout_rx);
                let stderr_receiver = ChannelReceiver::new(stderr_rx);
                let exit_code_receiver = ValueReceiver::new(exit_code_rx);

                Ok(PipelineData::ExternalStream {
                    stdout: if redirect_stdout || redirect_combine {
                        Some(RawStream::new(
                            Box::new(stdout_receiver),
                            output_ctrlc.clone(),
                            head,
                            None,
                        ))
                    } else {
                        None
                    },
                    stderr: if redirect_stderr {
                        Some(RawStream::new(
                            Box::new(stderr_receiver),
                            output_ctrlc.clone(),
                            head,
                            None,
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
                    trim_end_newline: self.trim_end_newline,
                })
            }
        }
    }

    pub fn create_process(
        &self,
        input: &PipelineData,
        use_cmd: bool,
        span: Span,
    ) -> Result<(CommandSys, Option<PipeReader>), ShellError> {
        let mut process = if let Some(d) = self.env_vars.get("PWD") {
            let mut process = if use_cmd {
                self.spawn_cmd_command(d)
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
        let reader = if self.redirect_combine {
            let (reader, writer) = os_pipe::pipe()?;
            let writer_clone = writer.try_clone()?;
            process.stdout(writer);
            process.stderr(writer_clone);
            Some(reader)
        } else {
            if self.redirect_stdout {
                process.stdout(Stdio::piped());
            }

            if self.redirect_stderr {
                process.stderr(Stdio::piped());
            }
            None
        };

        // If there is an input from the pipeline. The stdin from the process
        // is piped so it can be used to send the input information
        if !input.is_nothing() {
            process.stdin(Stdio::piped());
        }

        Ok((process, reader))
    }

    fn create_command(&self, cwd: &str) -> Result<CommandSys, ShellError> {
        // in all the other cases shell out
        if cfg!(windows) {
            //TODO. This should be modifiable from the config file.
            // We could give the option to call from powershell
            // for minimal builds cwd is unused
            if self.name.item.ends_with(".cmd") || self.name.item.ends_with(".bat") {
                Ok(self.spawn_cmd_command(cwd))
            } else {
                self.spawn_simple_command(cwd)
            }
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
            trim_expand_and_apply_arg(&mut process, arg, arg_keep_raw, cwd);
        }

        Ok(process)
    }

    /// Spawn a cmd command with `cmd /c args...`
    pub fn spawn_cmd_command(&self, cwd: &str) -> std::process::Command {
        let mut process = std::process::Command::new("cmd");

        // Disable AutoRun
        // TODO: There should be a config option to enable/disable this
        // Alternatively (even better) a config option to specify all the arguments to pass to cmd
        process.arg("/D");

        process.arg("/c");
        process.arg(&self.name.item);
        for (arg, arg_keep_raw) in self.args.iter().zip(self.arg_keep_raw.iter()) {
            // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
            // cmd.exe needs to have a caret to escape a pipe
            let arg = Spanned {
                item: arg.item.replace('|', "^|"),
                span: arg.span,
            };

            trim_expand_and_apply_arg(&mut process, &arg, arg_keep_raw, cwd)
        }

        process
    }
}

fn trim_expand_and_apply_arg(
    process: &mut CommandSys,
    arg: &Spanned<String>,
    arg_keep_raw: &bool,
    cwd: &str,
) {
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
    if !keep_raw {
        arg.item = nu_path::expand_tilde(arg.item)
            .to_string_lossy()
            .to_string();
    }
    let cwd = PathBuf::from(cwd);
    if arg.item.contains('*') && run_glob_expansion {
        if let Ok((prefix, matches)) = nu_engine::glob_from(&arg, &cwd, arg.span, None) {
            let matches: Vec<_> = matches.collect();

            // FIXME: do we want to special-case this further? We might accidentally expand when they don't
            // intend to
            if matches.is_empty() {
                process.arg(&arg.item);
            }
            for m in matches {
                if let Ok(arg) = m {
                    let arg = if let Some(prefix) = &prefix {
                        if let Ok(remainder) = arg.strip_prefix(prefix) {
                            let new_prefix = if let Some(pfx) = diff_paths(prefix, &cwd) {
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

/// This function returns a tuple with 3 items:
/// 1st item: trimmed string.
/// 2nd item: a boolean value indicate if it's ok to run glob expansion.
/// 3rd item: a boolean value indicate if we need to keep raw string.
fn trim_enclosing_quotes(input: &str) -> (String, bool, bool) {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('"'), Some('"')) => (chars.collect(), false, true),
        (Some('\''), Some('\'')) => (chars.collect(), false, true),
        // We treat back-quoted strings as bare words, so there's no need to keep them as raw strings
        (Some('`'), Some('`')) => (chars.collect(), true, false),
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

        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            break;
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
