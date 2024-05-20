use nu_cmd_base::hook::eval_hook;
use nu_engine::{command_prelude::*, env_to_strings, get_eval_expression};
use nu_protocol::{
    ast::{Expr, Expression},
    did_you_mean,
    process::ChildProcess,
    ByteStream, NuGlob, OutDest,
};
use nu_system::ForegroundChild;
use nu_utils::IgnoreCaseExt;
use os_pipe::PipeReader;
use pathdiff::diff_paths;
use regex::Regex;
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Command as CommandSys, Stdio},
    sync::Arc,
    thread,
};

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
            .required("command", SyntaxShape::String, "External command to run.")
            .rest("args", SyntaxShape::Any, "Arguments for external command.")
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cwd = engine_state.cwd(Some(stack))?;

        // Find the absolute path to the executable. On Windows, set the
        // executable to "cmd.exe" if it's is a CMD internal command. If the
        // command is not found, display a helpful error message.
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let executable = if cfg!(windows) && is_cmd_internal_commmand(&name.item) {
            PathBuf::from("cmd.exe")
        } else {
            let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
            let Some(executable) = which(&name.item, &paths, &cwd) else {
                return Err(command_not_found(
                    &name.item,
                    call.head,
                    engine_state,
                    stack,
                ));
            };
            executable
        };

        // Create the command.
        let mut command = std::process::Command::new(executable);

        // Configure PWD.
        command.current_dir(cwd);

        // Configure environment variables.
        let envs = env_to_strings(engine_state, stack)?;
        command.env_clear();
        command.envs(envs);

        // Configure args.
        let args = eval_arguments_from_call(engine_state, stack, call)?;
        if cfg!(windows) && is_cmd_internal_commmand(&name.item) {
            // The /D flag disables execution of AutoRun commands from registry.
            // The /C flag followed by a command name instructs CMD to execute
            // that command and quit.
            command.args(["/D", "/C", &name.item]);
            // Check for special characters in `args` and reject them.
            for arg in &args {
                if has_cmd_special_character(&arg.item) {
                    return Err(ShellError::ExternalCommand {
                        label: "Special characters are not allowed in CMD builtins".into(),
                        help: r#"These characters are special in CMD: / \ < > " | & ^"#.into(),
                        span: arg.span,
                    });
                }
            }
        }
        command.args(args.into_iter().map(|s| s.item));

        // Configure stdout and stderr. If both are set to `OutDest::Pipe`,
        // we'll setup a pipe that merge two streams into one.
        let stdout = stack.stdout();
        let stderr = stack.stderr();
        let merged_stream = if matches!(stdout, OutDest::Pipe) && matches!(stderr, OutDest::Pipe) {
            let (reader, writer) = os_pipe::pipe()?;
            command.stdout(writer.try_clone()?);
            command.stderr(writer);
            Some(reader)
        } else {
            command.stdout(Stdio::try_from(stdout)?);
            command.stderr(Stdio::try_from(stderr)?);
            None
        };

        // Configure stdin. We'll try connecting input to the child process
        // directly. If that's not possible, we'll setup a pipe and spawn a
        // thread to copy data into the child process.
        let data_to_copy_into_stdin = match input {
            PipelineData::ByteStream(stream, metadata) => match stream.into_stdio() {
                Ok(stdin) => {
                    command.stdin(stdin);
                    None
                }
                Err(stream) => {
                    command.stdin(Stdio::piped());
                    Some(PipelineData::ByteStream(stream, metadata))
                }
            },
            PipelineData::Empty => {
                command.stdin(Stdio::inherit());
                None
            }
            value => {
                command.stdin(Stdio::piped());
                Some(value)
            }
        };

        // Spawn the child process. On Unix, also put the child process to
        // foreground if we're in an interactive session.
        #[cfg(windows)]
        let mut child = ForegroundChild::spawn(command)?;
        #[cfg(unix)]
        let mut child = ForegroundChild::spawn(
            command,
            engine_state.is_interactive,
            &engine_state.pipeline_externals_state,
        )?;

        // If we need to copy data into the child process, do it now.
        if let Some(data) = data_to_copy_into_stdin {
            let stdin = child.as_mut().stdin.take().expect("stdin is piped");
            thread::spawn(move || {
                let _ = write_pipeline_data(data, stdin);
            });
        }

        // Wrap the output into a `PipelineData::ByteStream`.
        let child = ChildProcess::new(
            child,
            merged_stream,
            matches!(stderr, OutDest::Pipe),
            call.head,
        )?;
        Ok(PipelineData::ByteStream(
            ByteStream::child(child, call.head),
            None,
        ))
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
                example: r#"run-external "echo" "-n" "hello" | split chars"#,
                result: None,
            },
            Example {
                description: "Redirect stderr from an external command into the pipeline",
                example: r#"run-external "nu" "-c" "print -e hello" e>| split chars"#,
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
) -> Result<ExternalCommand, ShellError> {
    let name: Spanned<String> = call.req(engine_state, stack, 0)?;

    // Translate environment variables from Values to Strings
    let env_vars_str = env_to_strings(engine_state, stack)?;

    fn value_as_spanned(value: Value) -> Result<Spanned<String>, ShellError> {
        let span = value.span();

        value
            .coerce_string()
            .map(|item| Spanned { item, span })
            .map_err(|_| ShellError::ExternalCommand {
                label: format!("Cannot convert {} to a string", value.get_type()),
                help: "All arguments to an external command need to be string-compatible".into(),
                span,
            })
    }

    let eval_expression = get_eval_expression(engine_state);

    let mut spanned_args = vec![];
    let mut arg_keep_raw = vec![];
    for (arg, spread) in call.rest_iter(1) {
        match eval_expression(engine_state, stack, arg)? {
            Value::List { vals, .. } => {
                if spread {
                    // turn all the strings in the array into params.
                    // Example: one_arg may be something like ["ls" "-a"]
                    // convert it to "ls" "-a"
                    for v in vals {
                        spanned_args.push(value_as_spanned(v)?);
                        // for arguments in list, it's always treated as a whole arguments
                        arg_keep_raw.push(true);
                    }
                } else {
                    return Err(ShellError::CannotPassListToExternal {
                        arg: String::from_utf8_lossy(engine_state.get_span_contents(arg.span))
                            .into(),
                        span: arg.span,
                    });
                }
            }
            val => {
                if spread {
                    return Err(ShellError::CannotSpreadAsList { span: arg.span });
                } else {
                    spanned_args.push(value_as_spanned(val)?);
                    match arg.expr {
                        // refer to `parse_dollar_expr` function
                        // the expression type of $variable_name, $"($variable_name)"
                        // will be Expr::StringInterpolation, Expr::FullCellPath
                        Expr::StringInterpolation(_) | Expr::FullCellPath(_) => {
                            arg_keep_raw.push(true)
                        }
                        _ => arg_keep_raw.push(false),
                    }
                }
            }
        }
    }

    Ok(ExternalCommand {
        name,
        args: spanned_args,
        arg_keep_raw,
        out: stack.stdout().clone(),
        err: stack.stderr().clone(),
        env_vars: env_vars_str,
    })
}

#[derive(Clone)]
pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub arg_keep_raw: Vec<bool>,
    pub out: OutDest,
    pub err: OutDest,
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

        #[cfg(windows)]
        let (child, reader, input) = {
            // We may need to run `create_process` again, so we have to clone the underlying
            // file or pipe in `input` here first.
            let (input_consumed, stdin) = match &input {
                PipelineData::ByteStream(stream, ..) => match stream.source() {
                    nu_protocol::ByteStreamSource::Read(_) => (false, Stdio::piped()),
                    nu_protocol::ByteStreamSource::File(file) => {
                        (true, file.try_clone().err_span(head)?.into())
                    }
                    nu_protocol::ByteStreamSource::Child(child) => {
                        if let Some(nu_protocol::process::ChildPipe::Pipe(pipe)) = &child.stdout {
                            (true, pipe.try_clone().err_span(head)?.into())
                        } else {
                            (false, Stdio::piped())
                        }
                    }
                },
                PipelineData::Empty => (false, Stdio::inherit()),
                _ => (false, Stdio::piped()),
            };

            let mut input = input;
            let (cmd, mut reader) = self.create_process(stdin, false, head)?;
            let child = match ForegroundChild::spawn(cmd) {
                Ok(child) => {
                    if input_consumed {
                        input = PipelineData::Empty;
                    }
                    Ok(child)
                }
                Err(err) => {
                    // Running external commands on Windows has 2 points of complication:
                    // 1. Some common Windows commands are actually built in to cmd.exe, not executables in their own right.
                    // 2. We need to let users run batch scripts etc. (.bat, .cmd) without typing their extension

                    // To support these situations, we have a fallback path that gets run if a command
                    // fails to be run as a normal executable:
                    // 1. "shell out" to cmd.exe if the command is a known cmd.exe internal command
                    // 2. Otherwise, use `which-rs` to look for batch files etc. then run those in cmd.exe

                    // set the default value, maybe we'll override it later
                    let mut child = Err(err);

                    // This has the full list of cmd.exe "internal" commands: https://ss64.com/nt/syntax-internal.html
                    // I (Reilly) went through the full list and whittled it down to ones that are potentially useful:
                    const CMD_INTERNAL_COMMANDS: [&str; 9] = [
                        "ASSOC", "CLS", "ECHO", "FTYPE", "MKLINK", "PAUSE", "START", "VER", "VOL",
                    ];
                    let command_name = &self.name.item;
                    let looks_like_cmd_internal = CMD_INTERNAL_COMMANDS
                        .iter()
                        .any(|&cmd| command_name.eq_ignore_ascii_case(cmd));

                    let (data, stdin) = extract_stdio(input);
                    input = data;

                    if looks_like_cmd_internal {
                        let (cmd, new_reader) = self.create_process(stdin, true, head)?;
                        reader = new_reader;
                        child = ForegroundChild::spawn(cmd);
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
                                            if !file_name
                                                .to_string_lossy()
                                                .eq_ignore_case(command_name)
                                            {
                                                // which-rs found an executable file with a slightly different name
                                                // than the one the user tried. Let's try running it
                                                let mut new_command = self.clone();
                                                new_command.name = Spanned {
                                                    item: file_name.to_string_lossy().to_string(),
                                                    span: self.name.span,
                                                };
                                                let (cmd, new_reader) = new_command
                                                    .create_process(stdin, true, head)?;
                                                reader = new_reader;
                                                child = ForegroundChild::spawn(cmd);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    child
                }
            };

            (child, reader, input)
        };

        #[cfg(unix)]
        let (child, reader, input) = {
            let (input, stdin) = extract_stdio(input);
            let (cmd, reader) = self.create_process(stdin, false, head)?;
            let child = ForegroundChild::spawn(
                cmd,
                engine_state.is_interactive,
                &engine_state.pipeline_externals_state,
            );
            (child, reader, input)
        };

        match child {
            Err(err) => {
                match err.kind() {
                    // If file not found, try suggesting alternative commands to the user
                    std::io::ErrorKind::NotFound => {
                        // recommend a replacement if the user tried a removed command
                        let command_name_lower = self.name.item.to_lowercase();
                        let removed_from_nu = crate::removed_commands();
                        if removed_from_nu.contains_key(&command_name_lower) {
                            let replacement = match removed_from_nu.get(&command_name_lower) {
                                Some(s) => s.clone(),
                                None => "".to_string(),
                            };
                            return Err(ShellError::RemovedCommand {
                                removed: command_name_lower,
                                replacement,
                                span: self.name.span,
                            });
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
                                let canary = "ENTERED_COMMAND_NOT_FOUND";
                                let stack = &mut stack.start_capture();
                                if stack.has_env_var(&engine_state, canary) {
                                    return Err(ShellError::ExternalCommand {
                                        label: "command_not_found handler could not be run".into(),
                                        help: "make sure the command_not_found closure itself does not use unknown commands".to_string(),
                                        span: self.name.span,
                                    });
                                }
                                stack.add_env_var(
                                    canary.to_string(),
                                    Value::bool(true, Span::unknown()),
                                );
                                match eval_hook(
                                    &mut engine_state,
                                    stack,
                                    None,
                                    vec![(
                                        "cmd_name".into(),
                                        Value::string(self.name.item.to_string(), self.name.span),
                                    )],
                                    &hook,
                                    "command_not_found",
                                ) {
                                    Ok(PipelineData::Value(Value::String { val, .. }, ..)) => {
                                        err_str = format!("{}\n{}", err_str, val);
                                    }

                                    Err(err) => {
                                        stack.remove_env_var(&engine_state, canary);
                                        return Err(err);
                                    }
                                    _ => {}
                                }
                                stack.remove_env_var(&engine_state, canary);
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
                    Arc::make_mut(&mut engine_state.config).use_ansi_coloring = false;

                    // Pipe input into the external command's stdin
                    if let Some(mut stdin_write) = child.as_mut().stdin.take() {
                        thread::Builder::new()
                            .name("external stdin worker".to_string())
                            .spawn(move || {
                                let input = match input {
                                    // Don't touch binary input or byte streams
                                    input @ PipelineData::ByteStream(..) => input,
                                    input @ PipelineData::Value(Value::Binary { .. }, ..) => input,
                                    input => {
                                        let stack = &mut stack.start_capture();
                                        // Attempt to render the input as a table before piping it to the external.
                                        // This is important for pagers like `less`;
                                        // they need to get Nu data rendered for display to users.
                                        //
                                        // TODO: should we do something different for list<string> inputs?
                                        // Users often expect those to be piped to *nix tools as raw strings separated by newlines
                                        crate::Table.run(
                                            &engine_state,
                                            stack,
                                            &Call::new(head),
                                            input,
                                        )?
                                    }
                                };

                                if let PipelineData::ByteStream(stream, ..) = input {
                                    stream.write_to(&mut stdin_write)?;
                                } else {
                                    for value in input.into_iter() {
                                        let buf = value.coerce_into_binary()?;
                                        stdin_write.write_all(&buf)?;
                                    }
                                }

                                Ok::<_, ShellError>(())
                            })
                            .err_span(head)?;
                    }
                }

                let child =
                    ChildProcess::new(child, reader, matches!(self.err, OutDest::Pipe), head)?;

                Ok(PipelineData::ByteStream(
                    ByteStream::child(child, head),
                    None,
                ))
            }
        }
    }

    pub fn create_process(
        &self,
        stdin: Stdio,
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
            return Err(ShellError::GenericError{
                error: "Current directory not found".into(),
                msg: "did not find PWD environment variable".into(),
                span: Some(span),
                help: Some(concat!(
                    "The environment variable 'PWD' was not found. ",
                    "It is required to define the current directory when running an external command."
                ).into()),
                inner:Vec::new(),
            });
        };

        process.envs(&self.env_vars);

        // If the external is not the last command, its output will get piped
        // either as a string or binary
        let reader = if matches!(self.out, OutDest::Pipe) && matches!(self.err, OutDest::Pipe) {
            let (reader, writer) = os_pipe::pipe()?;
            let writer_clone = writer.try_clone()?;
            process.stdout(writer);
            process.stderr(writer_clone);
            Some(reader)
        } else {
            process.stdout(Stdio::try_from(&self.out)?);
            process.stderr(Stdio::try_from(&self.err)?);
            None
        };

        process.stdin(stdin);

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
    ///
    /// Note that this function will not set the cwd or environment variables.
    /// It only creates the command and adds arguments.
    pub fn spawn_simple_command(&self, cwd: &str) -> Result<std::process::Command, ShellError> {
        let (head, _, _) = trim_enclosing_quotes(&self.name.item);
        let head = nu_path::expand_to_real_path(head)
            .to_string_lossy()
            .to_string();

        let mut process = std::process::Command::new(head);
        process.env_clear();

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
    let (trimmed_args, mut run_glob_expansion, mut keep_raw) = trim_enclosing_quotes(&arg.item);
    if *arg_keep_raw {
        keep_raw = true;
        // it's a list or a variable, don't run glob expansion either
        run_glob_expansion = false;
    }
    let mut arg = Spanned {
        item: if keep_raw {
            trimmed_args
        } else {
            remove_quotes_old(trimmed_args)
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
        // we need to run glob expansion, so it's NeedExpand.
        let path = Spanned {
            item: NuGlob::Expand(arg.item.clone()),
            span: arg.span,
        };
        if let Ok((prefix, matches)) = nu_engine::glob_from(&path, &cwd, arg.span, None) {
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
    let command_folded_case = attempted_command.to_folded_case();
    let search_term_match = commands.iter().find(|sig| {
        sig.search_terms
            .iter()
            .any(|term| term.to_folded_case() == command_folded_case)
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

fn remove_quotes_old(input: String) -> String {
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

fn extract_stdio(pipeline: PipelineData) -> (PipelineData, Stdio) {
    match pipeline {
        PipelineData::ByteStream(stream, metadata) => match stream.into_stdio() {
            Ok(pipe) => (PipelineData::Empty, pipe),
            Err(stream) => (PipelineData::ByteStream(stream, metadata), Stdio::piped()),
        },
        PipelineData::Empty => (PipelineData::Empty, Stdio::inherit()),
        data => (data, Stdio::piped()),
    }
}

/// Removes surrounding quotes from a string. Doesn't remove quotes from raw
/// strings. Returns the original string if it doesn't have matching quotes.
fn remove_quotes(s: &str) -> &str {
    let quoted_by_double_quotes = s.len() >= 2 && s.starts_with('"') && s.ends_with('"');
    let quoted_by_single_quotes = s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'');
    let quoted_by_backticks = s.len() >= 2 && s.starts_with('`') && s.ends_with('`');
    if quoted_by_double_quotes || quoted_by_single_quotes || quoted_by_backticks {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Evaluate all arguments from a call, performing expansions when necessary.
pub fn eval_arguments_from_call(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Vec<Spanned<String>>, ShellError> {
    let cwd = engine_state.cwd(Some(stack))?;
    let mut args: Vec<Spanned<String>> = vec![];
    for (expr, spread) in call.rest_iter(1) {
        if is_bare_string(expr) {
            // If `expr` is a bare string, perform tilde-expansion,
            // glob-expansion, and inner-quotes-removal, in that order.
            for arg in eval_argument(engine_state, stack, expr, spread)? {
                let tilde_expanded = expand_tilde(&arg);
                for glob_expanded in expand_glob(&tilde_expanded, &cwd, expr.span)? {
                    let inner_quotes_removed = remove_inner_quotes(glob_expanded);
                    args.push(inner_quotes_removed.into_spanned(expr.span));
                }
            }
        } else {
            for arg in eval_argument(engine_state, stack, expr, spread)? {
                args.push(arg.into_spanned(expr.span));
            }
        }
    }
    Ok(args)
}

/// Evaluates an expression, coercing the values to strings.
///
/// Note: The parser currently has a special hack that retains surrounding
/// quotes for string literals in `Expression`, so that we can decide whether
/// the expression is considered a bare string. The hack doesn't affact string
/// literals within lists or records. This function will remove the quotes
/// before evaluating the expression.
fn eval_argument(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    spread: bool,
) -> Result<Vec<String>, ShellError> {
    // Remove quotes from string literals.
    let mut expr = expr.clone();
    if let Expr::String(s) = &expr.expr {
        expr.expr = Expr::String(remove_quotes(s).into());
    }

    let eval = get_eval_expression(engine_state);
    match eval(engine_state, stack, &expr)? {
        Value::List { vals, .. } => {
            if spread {
                vals.into_iter().map(|val| val.coerce_string()).collect()
            } else {
                Err(ShellError::CannotPassListToExternal {
                    arg: String::from_utf8_lossy(engine_state.get_span_contents(expr.span)).into(),
                    span: expr.span,
                })
            }
        }
        value => {
            if spread {
                Err(ShellError::CannotSpreadAsList { span: expr.span })
            } else {
                Ok(vec![value.coerce_string()?])
            }
        }
    }
}

/// Returns whether an expression is considered a bare string.
///
/// Bare strings are defined as string literals that are either unquoted or
/// quoted by backticks. Raw strings or string interpolations don't count.
fn is_bare_string(expr: &Expression) -> bool {
    let Expr::String(s) = &expr.expr else {
        return false;
    };
    let quoted_by_double_quotes = s.len() >= 2 && s.starts_with('"') && s.ends_with('"');
    let quoted_by_single_quotes = s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'');
    !quoted_by_double_quotes && !quoted_by_single_quotes
}

/// Performs tilde expansion on `arg`. Returns the original string if `arg`
/// doesn't start with tilde.
fn expand_tilde(arg: &str) -> String {
    nu_path::expand_tilde(arg).to_string_lossy().to_string()
}

/// Performs glob expansion on `arg`. If the expansion found no matches, returns
/// the original string as the expansion result.
///
/// Note: This matches the default behavior of Bash, but is known to be
/// error-prone. We might want to change this behavior in the future.
fn expand_glob(arg: &str, cwd: &Path, span: Span) -> Result<Vec<String>, ShellError> {
    let paths =
        nu_glob::glob_with_parent(arg, nu_glob::MatchOptions::default(), cwd).map_err(|err| {
            ShellError::InvalidGlobPattern {
                msg: err.msg.to_string(),
                span,
            }
        })?;

    let mut result = vec![];
    for path in paths {
        let path = path.map_err(|err| ShellError::IOErrorSpanned {
            msg: format!("{}: {:?}", err.path().display(), err.error()),
            span,
        })?;
        // Strip PWD from the resulting paths if possible.
        let path_stripped = path.strip_prefix(cwd).unwrap_or(&path);
        let path_string = path_stripped.to_string_lossy().to_string();
        result.push(path_string);
    }

    if result.is_empty() {
        result.push(arg.to_string());
    }

    Ok(result)
}

/// Transforms `--option="value"` into `--option=value`. `value` can be quoted
/// with double or single quotes. The original string should have an `=` sign,
/// otherwise this function returns the original string. Does not resolve escape
/// sequences within `value`. Only removes the first matching pair of quotes.
fn remove_inner_quotes(arg: impl Into<String>) -> String {
    let arg = arg.into();
    let re = Regex::new(r#"^(?<option>.*?)=['"](?<value>.*)['"]$"#).expect("valid regex");
    if let Some(caps) = re.captures(&arg) {
        format!("{}={}", &caps["option"], &caps["value"])
    } else {
        arg
    }
}

/// Write `PipelineData` into `writer`. If `PipelineData` is not binary, it is
/// first rendered using the `table` command.
///
/// Note: Avoid using this function when piping data from an external command to
/// another external command, because it copies data unnecessarily. Instead,
/// extract the pipe from the `PipelineData::ByteStream` of the first command
/// and hand it to the second command directly.
fn write_pipeline_data(data: PipelineData, mut writer: impl Write) -> Result<(), ShellError> {
    if let PipelineData::ByteStream(stream, ..) = data {
        stream.write_to(&mut writer)?;
    } else if let PipelineData::Value(Value::Binary { val, .. }, ..) = data {
        writer.write_all(&val)?;
    } else {
        let mut engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.start_capture();

        // Turn off color as we pass data through
        Arc::make_mut(&mut engine_state.config).use_ansi_coloring = false;

        // Invoke the `table` command.
        let output =
            crate::Table.run(&engine_state, &mut stack, &Call::new(Span::unknown()), data)?;

        // Write the output.
        for value in output {
            let bytes = value.coerce_into_binary()?;
            writer.write_all(&bytes)?;
        }
    }
    Ok(())
}

/// Returns a helpful error message given an invalid command name,
pub fn command_not_found(
    name: &str,
    span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> ShellError {
    // Run the `command_not_found` hook if there is one.
    if let Some(hook) = &engine_state.config.hooks.command_not_found {
        let mut stack = stack.start_capture();
        // Set a special environment variable to avoid infinite loops when the
        // `command_not_found` hook triggers itself.
        let canary = "ENTERED_COMMAND_NOT_FOUND";
        if stack.has_env_var(engine_state, canary) {
            return ShellError::ExternalCommand {
                label: format!(
                    "Command {name} not found while running the `command_not_found` hook"
                ),
                help: "Make sure the `command_not_found` hook itself does not use unknown commands"
                    .into(),
                span,
            };
        }
        stack.add_env_var(canary.into(), Value::bool(true, Span::unknown()));

        let output = eval_hook(
            &mut engine_state.clone(),
            &mut stack,
            None,
            vec![("cmd_name".into(), Value::string(name, span))],
            hook,
            "command_not_found",
        );

        // Remove the special environment variable that we just set.
        stack.remove_env_var(engine_state, canary);

        match output {
            Ok(PipelineData::Value(Value::String { val, .. }, ..)) => {
                return ShellError::ExternalCommand {
                    label: format!("Command `{name}` not found"),
                    help: val,
                    span,
                };
            }
            Err(err) => {
                return err;
            }
            _ => {
                // The hook did not return a string, so ignore it.
            }
        }
    }

    // If the name is one of the removed commands, recommend a replacement.
    if let Some(replacement) = crate::removed_commands().get(&name.to_lowercase()) {
        return ShellError::RemovedCommand {
            removed: name.to_lowercase(),
            replacement: replacement.clone(),
            span,
        };
    }

    // The command might be from another module. Try to find it.
    if let Some(module) = engine_state.which_module_has_decl(name.as_bytes(), &[]) {
        let module = String::from_utf8_lossy(module);
        // Is the command already imported?
        let full_name = format!("{module} {name}");
        if engine_state.find_decl(full_name.as_bytes(), &[]).is_some() {
            return ShellError::ExternalCommand {
                label: format!("Command `{name}` not found"),
                help: format!("Did you mean `{full_name}`?"),
                span,
            };
        } else {
            return ShellError::ExternalCommand {
                label: format!("Command `{name}` not found"),
                help: format!("A command with that name exists in module `{module}`. Try importing it with `use`"),
                span,
            };
        }
    }

    // Try to match the name with the search terms of existing commands.
    let signatures = engine_state.get_signatures(false);
    if let Some(sig) = signatures.iter().find(|sig| {
        sig.search_terms
            .iter()
            .any(|term| term.to_folded_case() == name.to_folded_case())
    }) {
        return ShellError::ExternalCommand {
            label: format!("Command `{name}` not found"),
            help: format!("Did you mean `{}`?", sig.name),
            span,
        };
    }

    // Try a fuzzy search on the names of all existing commands.
    if let Some(cmd) = did_you_mean(signatures.iter().map(|sig| &sig.name), name) {
        // The user is invoking an external command with the same name as a
        // built-in command. Remind them of this.
        if cmd == name {
            return ShellError::ExternalCommand {
                label: format!("Command `{name}` not found"),
                help: "There is a built-in command with the same name".into(),
                span,
            };
        }
        return ShellError::ExternalCommand {
            label: format!("Command `{name}` not found"),
            help: format!("Did you mean `{cmd}`?"),
            span,
        };
    }

    // We found nothing useful. Give up and return a generic error message.
    ShellError::ExternalCommand {
        label: format!("Command `{name}` not found"),
        help: "".into(),
        span,
    }
}

/// Searches for the absolute path of an executable by name.
///
/// This is a wrapper around `which::which_in()` except that, on Windows, it
/// also searches the current directory before any PATH entries.
///
/// Implementation note: the `which.rs` crate always uses PATHEXT from the
/// environment. As such, changing PATHEXT within Nushell doesn't work without
/// updating the actual environment of the Nushell process.
pub fn which(name: &str, paths: &str, cwd: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let paths = format!("{};{}", cwd.display(), paths);
    which::which_in(name, Some(paths), cwd).ok()
}

/// Returns true if `name` is a (somewhat useful) CMD internal command.
fn is_cmd_internal_commmand(name: &str) -> bool {
    const COMMANDS: &[&str] = &[
        "ASSOC", "CLS", "ECHO", "FTYPE", "MKLINK", "PAUSE", "START", "VER", "VOL",
    ];
    COMMANDS.iter().any(|cmd| cmd.eq_ignore_ascii_case(name))
}

/// Returns true if a string contains CMD special characters.
fn has_cmd_special_character(s: &str) -> bool {
    const SPECIAL_CHARS: &[char] = &['/', '\\', '<', '>', '"', '|', '&', '^'];
    SPECIAL_CHARS.iter().any(|c| s.contains(*c))
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::ast::ListItem;

    #[test]
    fn test_remove_quotes() {
        assert_eq!(remove_quotes(r#""#), r#""#);
        assert_eq!(remove_quotes(r#"'"#), r#"'"#);
        assert_eq!(remove_quotes(r#"''"#), r#""#);
        assert_eq!(remove_quotes(r#""foo""#), r#"foo"#);
        assert_eq!(remove_quotes(r#"`foo '"' bar`"#), r#"foo '"' bar"#);
        assert_eq!(remove_quotes(r#"'foo' bar"#), r#"'foo' bar"#);
        assert_eq!(remove_quotes(r#"r#'foo'#"#), r#"r#'foo'#"#);
    }

    #[test]
    fn test_eval_argument() {
        fn expression(expr: Expr) -> Expression {
            Expression {
                expr: expr,
                span: Span::unknown(),
                ty: Type::Any,
                custom_completion: None,
            }
        }

        fn eval(expr: Expr, spread: bool) -> Result<Vec<String>, ShellError> {
            let engine_state = EngineState::new();
            let mut stack = Stack::new();
            eval_argument(&engine_state, &mut stack, &expression(expr), spread)
        }

        let actual = eval(Expr::String("".into()), false).unwrap();
        let expected = &[""];
        assert_eq!(actual, expected);

        let actual = eval(Expr::String("'foo'".into()), false).unwrap();
        let expected = &["foo"];
        assert_eq!(actual, expected);

        let actual = eval(Expr::RawString("'foo'".into()), false).unwrap();
        let expected = &["'foo'"];
        assert_eq!(actual, expected);

        let actual = eval(Expr::List(vec![]), true).unwrap();
        let expected: &[&str] = &[];
        assert_eq!(actual, expected);

        let actual = eval(
            Expr::List(vec![
                ListItem::Item(expression(Expr::String("'foo'".into()))),
                ListItem::Item(expression(Expr::String("bar".into()))),
            ]),
            true,
        )
        .unwrap();
        let expected = &["'foo'", "bar"];
        assert_eq!(actual, expected);

        eval(Expr::String("".into()), true).unwrap_err();
        eval(Expr::List(vec![]), false).unwrap_err();
    }

    #[test]
    fn test_expand_glob() {
        let tempdir = tempfile::tempdir().unwrap();
        let cwd = tempdir.path();
        std::fs::File::create(cwd.join("a.txt")).unwrap();
        std::fs::File::create(cwd.join("b.txt")).unwrap();

        let actual = expand_glob("*.txt", cwd, Span::unknown()).unwrap();
        let expected = &["a.txt", "b.txt"];
        assert_eq!(actual, expected);

        let actual = expand_glob("'*.txt'", cwd, Span::unknown()).unwrap();
        let expected = &["'*.txt'"];
        assert_eq!(actual, expected);

        expand_glob("[*.txt", cwd, Span::unknown()).unwrap_err();
    }

    #[test]
    fn test_remove_inner_quotes() {
        let actual = remove_inner_quotes(r#"--option=value"#);
        let expected = r#"--option=value"#;
        assert_eq!(actual, expected);

        let actual = remove_inner_quotes(r#"--option="value""#);
        let expected = r#"--option=value"#;
        assert_eq!(actual, expected);

        let actual = remove_inner_quotes(r#"--option='value'"#);
        let expected = r#"--option=value"#;
        assert_eq!(actual, expected);

        let actual = remove_inner_quotes(r#"--option "value""#);
        let expected = r#"--option "value""#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_write_pipeline_data() {
        let mut buf = vec![];
        let input = PipelineData::Empty;
        write_pipeline_data(input, &mut buf).unwrap();
        assert_eq!(buf, b"");

        let mut buf = vec![];
        let input = PipelineData::Value(Value::string("foo", Span::unknown()), None);
        write_pipeline_data(input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");

        let mut buf = vec![];
        let input = PipelineData::Value(Value::binary(b"foo", Span::unknown()), None);
        write_pipeline_data(input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");

        let mut buf = vec![];
        let input = PipelineData::ByteStream(
            ByteStream::read(
                b"foo".as_slice(),
                Span::unknown(),
                None,
                ByteStreamType::Unknown,
            ),
            None,
        );
        write_pipeline_data(input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");
    }
}
