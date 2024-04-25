use nu_cmd_base::hook::eval_hook;
use nu_engine::{command_prelude::*, env_to_strings, get_eval_expression};
use nu_protocol::{ast::Expr, did_you_mean, ListStream, NuGlob, OutDest, RawStream};
use nu_system::ForegroundChild;
use nu_utils::IgnoreCaseExt;
use os_pipe::PipeReader;
use pathdiff::diff_paths;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command as CommandSys, Stdio},
    sync::{mpsc, Arc},
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
        let command = create_external_command(engine_state, stack, call)?;
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

        let ctrlc = engine_state.ctrlc.clone();

        #[allow(unused_mut)]
        let (cmd, mut reader) = self.create_process(&input, false, head)?;

        #[cfg(all(not(unix), not(windows)))] // are there any systems like this?
        let child = ForegroundChild::spawn(cmd);

        #[cfg(windows)]
        let child = match ForegroundChild::spawn(cmd) {
            Ok(child) => Ok(child),
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

                if looks_like_cmd_internal {
                    let (cmd, new_reader) = self.create_process(&input, true, head)?;
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
                                        if !file_name.to_string_lossy().eq_ignore_case(command_name)
                                        {
                                            // which-rs found an executable file with a slightly different name
                                            // than the one the user tried. Let's try running it
                                            let mut new_command = self.clone();
                                            new_command.name = Spanned {
                                                item: file_name.to_string_lossy().to_string(),
                                                span: self.name.span,
                                            };
                                            let (cmd, new_reader) =
                                                new_command.create_process(&input, true, head)?;
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

        #[cfg(unix)]
        let child = ForegroundChild::spawn(
            cmd,
            engine_state.is_interactive,
            &engine_state.pipeline_externals_state,
        );

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
                                    input @ PipelineData::Value(Value::Binary { .. }, ..) => {
                                        Ok(input)
                                    }
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
                                        )
                                    }
                                };

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
                            .err_span(head)?;
                    }
                }

                #[cfg(unix)]
                let commandname = self.name.item.clone();
                let span = self.name.span;
                let (exit_code_tx, exit_code_rx) = mpsc::channel();

                let (stdout, stderr) = if let Some(combined) = reader {
                    (
                        Some(RawStream::new(
                            Box::new(ByteLines::new(combined)),
                            ctrlc.clone(),
                            head,
                            None,
                        )),
                        None,
                    )
                } else {
                    let stdout = child.as_mut().stdout.take().map(|out| {
                        RawStream::new(Box::new(ByteLines::new(out)), ctrlc.clone(), head, None)
                    });

                    let stderr = child.as_mut().stderr.take().map(|err| {
                        RawStream::new(Box::new(ByteLines::new(err)), ctrlc.clone(), head, None)
                    });

                    if matches!(self.err, OutDest::Pipe) {
                        (stderr, stdout)
                    } else {
                        (stdout, stderr)
                    }
                };

                // Create a thread to wait for an exit code.
                thread::Builder::new()
                    .name("exit code waiter".into())
                    .spawn(move || match child.as_mut().wait() {
                        Err(err) => Err(ShellError::ExternalCommand {
                            label: "External command exited with error".into(),
                            help: err.to_string(),
                            span,
                        }),
                        Ok(x) => {
                            #[cfg(unix)]
                            {
                                use nix::sys::signal::Signal;
                                use nu_ansi_term::{Color, Style};
                                use std::os::unix::process::ExitStatusExt;

                                if x.core_dumped() {
                                    let cause = x
                                        .signal()
                                        .and_then(|sig| {
                                            Signal::try_from(sig).ok().map(Signal::as_str)
                                        })
                                        .unwrap_or("Something went wrong");

                                    let style = Style::new().bold().on(Color::Red);
                                    let message = format!(
                                        "{cause}: child process '{commandname}' core dumped"
                                    );
                                    eprintln!("{}", style.paint(&message));
                                    let _ = exit_code_tx.send(Value::error(
                                        ShellError::ExternalCommand {
                                            label: "core dumped".into(),
                                            help: message,
                                            span: head,
                                        },
                                        head,
                                    ));
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
                    })
                    .err_span(head)?;

                let exit_code_receiver = ValueReceiver::new(exit_code_rx);

                Ok(PipelineData::ExternalStream {
                    stdout,
                    stderr,
                    exit_code: Some(ListStream::from_stream(
                        Box::new(exit_code_receiver),
                        ctrlc.clone(),
                    )),
                    span: head,
                    metadata: None,
                    trim_end_newline: true,
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

struct ByteLines<R: Read>(BufReader<R>);

impl<R: Read> ByteLines<R> {
    fn new(read: R) -> Self {
        Self(BufReader::new(read))
    }
}

impl<R: Read> Iterator for ByteLines<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        // `read_until` will never stop reading unless `\n` or EOF is encountered,
        // so let's limit the number of bytes using `take` as the Rust docs suggest.
        let capacity = self.0.capacity() as u64;
        let mut reader = (&mut self.0).take(capacity);
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buf)),
            Err(e) => Some(Err(e.into())),
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
        let input = r#"sh -c 'echo a'"#.into();
        let res = remove_quotes(input);

        assert_eq!("sh -c 'echo a'", res)
    }
}
