use nu_cmd_base::hook::eval_hook;
use nu_engine::{command_prelude::*, env_to_strings};
use nu_path::{AbsolutePath, dots::expand_ndots_safe, expand_tilde};
use nu_protocol::{
    ByteStream, NuGlob, OutDest, Signals, UseAnsiColoring, did_you_mean,
    process::{ChildProcess, PostWaitCallback},
    shell_error::io::IoError,
};
use nu_system::{ForegroundChild, kill_by_pid};
use nu_utils::IgnoreCaseExt;
use pathdiff::diff_paths;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    thread,
};

#[derive(Clone)]
pub struct External;

impl Command for External {
    fn name(&self) -> &str {
        "run-external"
    }

    fn description(&self) -> &str {
        "Runs external command."
    }

    fn extra_description(&self) -> &str {
        r#"All externals are run with this command, whether you call it directly with `run-external external` or use `external` or `^external`.
If you create a custom command with this name, that will be used instead."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .rest(
                "command",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Any]),
                "External command to run, with arguments.",
            )
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
        let rest = call.rest::<Value>(engine_state, stack, 0)?;
        let name_args = rest.split_first().map(|(x, y)| (x, y.to_vec()));

        let Some((name, mut call_args)) = name_args else {
            return Err(ShellError::MissingParameter {
                param_name: "no command given".into(),
                span: call.head,
            });
        };

        let name_str: Cow<str> = match &name {
            Value::Glob { val, .. } => Cow::Borrowed(val),
            Value::String { val, .. } => Cow::Borrowed(val),
            Value::List { vals, .. } => {
                let Some((first, args)) = vals.split_first() else {
                    return Err(ShellError::MissingParameter {
                        param_name: "external command given as list empty".into(),
                        span: call.head,
                    });
                };
                // Prepend elements in command list to the list of arguments except the first
                call_args.splice(0..0, args.to_vec());
                first.coerce_str()?
            }
            _ => Cow::Owned(name.clone().coerce_into_string()?),
        };

        let expanded_name = match &name {
            // Expand tilde and ndots on the name if it's a bare string / glob (#13000)
            Value::Glob { no_expand, .. } if !*no_expand => {
                expand_ndots_safe(expand_tilde(&*name_str))
            }
            _ => Path::new(&*name_str).to_owned(),
        };

        let paths = nu_engine::env::path_str(engine_state, stack, call.head).unwrap_or_default();

        // On Windows, the user could have run the cmd.exe built-in commands "assoc"
        // and "ftype" to create a file association for an arbitrary file extension.
        // They then could have added that extension to the PATHEXT environment variable.
        // For example, a nushell script with extension ".nu" can be set up with
        // "assoc .nu=nuscript" and "ftype nuscript=C:\path\to\nu.exe '%1' %*",
        // and then by adding ".NU" to PATHEXT. In this case we use the which command,
        // which will find the executable with or without the extension. If "which"
        // returns true, that means that we've found the script and we believe the
        // user wants to use the windows association to run the script. The only
        // easy way to do this is to run cmd.exe with the script as an argument.
        // File extensions of .COM, .EXE, .BAT, and .CMD are ignored because Windows
        // can run those files directly. PS1 files are also ignored and that
        // extension is handled in a separate block below.
        let pathext_script_in_windows = if cfg!(windows) {
            if let Some(executable) = which(&expanded_name, &paths, cwd.as_ref()) {
                let ext = executable
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_uppercase();

                !["COM", "EXE", "BAT", "CMD", "PS1"]
                    .iter()
                    .any(|c| *c == ext)
            } else {
                false
            }
        } else {
            false
        };

        // let's make sure it's a .ps1 script, but only on Windows
        let (potential_powershell_script, path_to_ps1_executable) = if cfg!(windows) {
            if let Some(executable) = which(&expanded_name, &paths, cwd.as_ref()) {
                let ext = executable
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_uppercase();
                (ext == "PS1", Some(executable))
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };

        // Find the absolute path to the executable. On Windows, set the
        // executable to "cmd.exe" if it's a CMD internal command. If the
        // command is not found, display a helpful error message.
        let executable = if cfg!(windows)
            && (is_cmd_internal_command(&name_str) || pathext_script_in_windows)
        {
            PathBuf::from("cmd.exe")
        } else if cfg!(windows) && potential_powershell_script && path_to_ps1_executable.is_some() {
            // If we're on Windows and we're trying to run a PowerShell script, we'll use
            // `powershell.exe` to run it. We shouldn't have to check for powershell.exe because
            // it's automatically installed on all modern windows systems.
            PathBuf::from("powershell.exe")
        } else {
            // Determine the PATH to be used and then use `which` to find it - though this has no
            // effect if it's an absolute path already
            let Some(executable) = which(&expanded_name, &paths, cwd.as_ref()) else {
                return Err(command_not_found(
                    &name_str,
                    call.head,
                    engine_state,
                    stack,
                    &cwd,
                ));
            };
            executable
        };

        // Create the command.
        let mut command = std::process::Command::new(&executable);

        // Configure PWD.
        command.current_dir(cwd);

        // Configure environment variables.
        let envs = env_to_strings(engine_state, stack)?;
        command.env_clear();
        command.envs(envs);

        // Configure args.
        let args = eval_external_arguments(engine_state, stack, call_args)?;
        #[cfg(windows)]
        if is_cmd_internal_command(&name_str) || pathext_script_in_windows {
            // The /D flag disables execution of AutoRun commands from registry.
            // The /C flag followed by a command name instructs CMD to execute
            // that command and quit.
            command.args(["/D", "/C", &expanded_name.to_string_lossy()]);
            for arg in &args {
                command.raw_arg(escape_cmd_argument(arg)?);
            }
        } else if potential_powershell_script {
            command.args([
                "-File",
                &path_to_ps1_executable.unwrap_or_default().to_string_lossy(),
            ]);
            command.args(args.into_iter().map(|s| s.item));
        } else {
            command.args(args.into_iter().map(|s| s.item));
        }
        #[cfg(not(windows))]
        command.args(args.into_iter().map(|s| s.item));

        // Configure stdout and stderr. If both are set to `OutDest::Pipe`,
        // we'll set up a pipe that merges two streams into one.
        let stdout = stack.stdout();
        let stderr = stack.stderr();
        let merged_stream = if matches!(stdout, OutDest::Pipe) && matches!(stderr, OutDest::Pipe) {
            let (reader, writer) =
                os_pipe::pipe().map_err(|err| IoError::new(err, call.head, None))?;
            command.stdout(
                writer
                    .try_clone()
                    .map_err(|err| IoError::new(err, call.head, None))?,
            );
            command.stderr(writer);
            Some(reader)
        } else {
            if engine_state.is_background_job()
                && matches!(stdout, OutDest::Inherit | OutDest::Print)
            {
                command.stdout(Stdio::null());
            } else {
                command.stdout(
                    Stdio::try_from(stdout).map_err(|err| IoError::new(err, call.head, None))?,
                );
            }

            if engine_state.is_background_job()
                && matches!(stderr, OutDest::Inherit | OutDest::Print)
            {
                command.stderr(Stdio::null());
            } else {
                command.stderr(
                    Stdio::try_from(stderr).map_err(|err| IoError::new(err, call.head, None))?,
                );
            }

            None
        };

        // Configure stdin. We'll try connecting input to the child process
        // directly. If that's not possible, we'll set up a pipe and spawn a
        // thread to copy data into the child process.
        let data_to_copy_into_stdin = match input {
            PipelineData::ByteStream(stream, metadata) => match stream.into_stdio() {
                Ok(stdin) => {
                    command.stdin(stdin);
                    None
                }
                Err(stream) => {
                    command.stdin(Stdio::piped());
                    Some(PipelineData::byte_stream(stream, metadata))
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

        // Log the command we're about to run in case it's useful for debugging purposes.
        log::trace!("run-external spawning: {command:?}");

        // Spawn the child process. On Unix, also put the child process to
        // foreground if we're in an interactive session.
        #[cfg(windows)]
        let child = ForegroundChild::spawn(command);
        #[cfg(unix)]
        let child = ForegroundChild::spawn(
            command,
            engine_state.is_interactive,
            engine_state.is_background_job(),
            &engine_state.pipeline_externals_state,
        );

        let mut child = child.map_err(|err| {
            let context = format!("Could not spawn foreground child: {err}");
            IoError::new_internal(err, context, nu_protocol::location!())
        })?;

        if let Some(thread_job) = engine_state.current_thread_job()
            && !thread_job.try_add_pid(child.pid())
        {
            kill_by_pid(child.pid().into()).map_err(|err| {
                ShellError::Io(IoError::new_internal(
                    err,
                    "Could not spawn external stdin worker",
                    nu_protocol::location!(),
                ))
            })?;
        }

        // If we need to copy data into the child process, do it now.
        if let Some(data) = data_to_copy_into_stdin {
            let stdin = child.as_mut().stdin.take().expect("stdin is piped");
            let engine_state = engine_state.clone();
            let stack = stack.clone();
            thread::Builder::new()
                .name("external stdin worker".into())
                .spawn(move || {
                    let _ = write_pipeline_data(engine_state, stack, data, stdin);
                })
                .map_err(|err| {
                    IoError::new_with_additional_context(
                        err,
                        call.head,
                        None,
                        "Could not spawn external stdin worker",
                    )
                })?;
        }

        let child_pid = child.pid();

        // Wrap the output into a `PipelineData::byte_stream`.
        let mut child = ChildProcess::new(
            child,
            merged_stream,
            matches!(stderr, OutDest::Pipe),
            call.head,
            Some(PostWaitCallback::for_job_control(
                engine_state,
                Some(child_pid),
                executable
                    .as_path()
                    .file_name()
                    .and_then(|it| it.to_str())
                    .map(|it| it.to_string()),
            )),
        )?;

        if matches!(stdout, OutDest::Pipe | OutDest::PipeSeparate)
            || matches!(stderr, OutDest::Pipe | OutDest::PipeSeparate)
        {
            child.ignore_error(true);
        }

        Ok(PipelineData::byte_stream(
            ByteStream::child(child, call.head),
            None,
        ))
    }

    fn examples(&self) -> Vec<Example<'_>> {
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

/// Evaluate all arguments, performing expansions when necessary.
pub fn eval_external_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call_args: Vec<Value>,
) -> Result<Vec<Spanned<OsString>>, ShellError> {
    let cwd = engine_state.cwd(Some(stack))?;
    let mut args: Vec<Spanned<OsString>> = Vec::with_capacity(call_args.len());

    for arg in call_args {
        let span = arg.span();
        match arg {
            // Expand globs passed to run-external
            Value::Glob { val, no_expand, .. } if !no_expand => args.extend(
                expand_glob(
                    &val,
                    cwd.as_std_path(),
                    span,
                    engine_state.signals().clone(),
                )?
                .into_iter()
                .map(|s| s.into_spanned(span)),
            ),
            other => args
                .push(OsString::from(coerce_into_string(engine_state, other)?).into_spanned(span)),
        }
    }
    Ok(args)
}

/// Custom `coerce_into_string()`, including globs, since those are often args to `run-external`
/// as well
fn coerce_into_string(engine_state: &EngineState, val: Value) -> Result<String, ShellError> {
    match val {
        Value::List { .. } => Err(ShellError::CannotPassListToExternal {
            arg: String::from_utf8_lossy(engine_state.get_span_contents(val.span())).into_owned(),
            span: val.span(),
        }),
        Value::Glob { val, .. } => Ok(val),
        _ => val.coerce_into_string(),
    }
}

/// Performs glob expansion on `arg`. If the expansion found no matches or the pattern
/// is not a valid glob, then this returns the original string as the expansion result.
///
/// Note: This matches the default behavior of Bash, but is known to be
/// error-prone. We might want to change this behavior in the future.
fn expand_glob(
    arg: &str,
    cwd: &Path,
    span: Span,
    signals: Signals,
) -> Result<Vec<OsString>, ShellError> {
    // For an argument that isn't a glob, just do the `expand_tilde`
    // and `expand_ndots` expansion
    if !nu_glob::is_glob(arg) {
        let path = expand_ndots_safe(expand_tilde(arg));
        return Ok(vec![path.into()]);
    }

    // We must use `nu_engine::glob_from` here, in order to ensure we get paths from the correct
    // dir
    let glob = NuGlob::Expand(arg.to_owned()).into_spanned(span);
    if let Ok((prefix, matches)) = nu_engine::glob_from(&glob, cwd, span, None, signals.clone()) {
        let mut result: Vec<OsString> = vec![];

        for m in matches {
            signals.check(&span)?;
            if let Ok(arg) = m {
                let arg = resolve_globbed_path_to_cwd_relative(arg, prefix.as_ref(), cwd);
                result.push(arg.into());
            } else {
                result.push(arg.into());
            }
        }

        // FIXME: do we want to special-case this further? We might accidentally expand when they don't
        // intend to
        if result.is_empty() {
            result.push(arg.into());
        }

        Ok(result)
    } else {
        Ok(vec![arg.into()])
    }
}

fn resolve_globbed_path_to_cwd_relative(
    path: PathBuf,
    prefix: Option<&PathBuf>,
    cwd: &Path,
) -> PathBuf {
    if let Some(prefix) = prefix {
        if let Ok(remainder) = path.strip_prefix(prefix) {
            let new_prefix = if let Some(pfx) = diff_paths(prefix, cwd) {
                pfx
            } else {
                prefix.to_path_buf()
            };
            new_prefix.join(remainder)
        } else {
            path
        }
    } else {
        path
    }
}

/// Write `PipelineData` into `writer`. If `PipelineData` is not binary, it is
/// first rendered using the `table` command.
///
/// Note: Avoid using this function when piping data from an external command to
/// another external command, because it copies data unnecessarily. Instead,
/// extract the pipe from the `PipelineData::byte_stream` of the first command
/// and hand it to the second command directly.
fn write_pipeline_data(
    mut engine_state: EngineState,
    mut stack: Stack,
    data: PipelineData,
    mut writer: impl Write,
) -> Result<(), ShellError> {
    if let PipelineData::ByteStream(stream, ..) = data {
        stream.write_to(writer)?;
    } else if let PipelineData::Value(Value::Binary { val, .. }, ..) = data {
        writer.write_all(&val).map_err(|err| {
            IoError::new_internal(
                err,
                "Could not write pipeline data",
                nu_protocol::location!(),
            )
        })?;
    } else {
        stack.start_collect_value();

        // Turn off color as we pass data through
        Arc::make_mut(&mut engine_state.config).use_ansi_coloring = UseAnsiColoring::False;

        // Invoke the `table` command.
        let output =
            crate::Table.run(&engine_state, &mut stack, &Call::new(Span::unknown()), data)?;

        // Write the output.
        for value in output {
            let bytes = value.coerce_into_binary()?;
            writer.write_all(&bytes).map_err(|err| {
                IoError::new_internal(
                    err,
                    "Could not write pipeline data",
                    nu_protocol::location!(),
                )
            })?;
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
    cwd: &AbsolutePath,
) -> ShellError {
    // Run the `command_not_found` hook if there is one.
    if let Some(hook) = &stack.get_config(engine_state).hooks.command_not_found {
        let mut stack = stack.start_collect_value();
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
                help: format!(
                    "A command with that name exists in module `{module}`. Try importing it with `use`"
                ),
                span,
            };
        }
    }

    // Try to match the name with the search terms of existing commands.
    let signatures = engine_state.get_signatures_and_declids(false);
    if let Some((sig, _)) = signatures.iter().find(|(sig, _)| {
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
    if let Some(cmd) = did_you_mean(signatures.iter().map(|(sig, _)| &sig.name), name) {
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

    // If we find a file, it's likely that the user forgot to set permissions
    if cwd.join(name).is_file() {
        return ShellError::ExternalCommand {
            label: format!("Command `{name}` not found"),
            help: format!(
                "`{name}` refers to a file that is not executable. Did you forget to set execute permissions?"
            ),
            span,
        };
    }

    // We found nothing useful. Give up and return a generic error message.
    ShellError::ExternalCommand {
        label: format!("Command `{name}` not found"),
        help: format!("`{name}` is neither a Nushell built-in or a known external command"),
        span,
    }
}

/// Searches for the absolute path of an executable by name. `.bat` and `.cmd`
/// files are recognized as executables on Windows.
///
/// This is a wrapper around `which::which_in()` except that, on Windows, it
/// also searches the current directory before any PATH entries.
///
/// Note: the `which.rs` crate always uses PATHEXT from the environment. As
/// such, changing PATHEXT within Nushell doesn't work without updating the
/// actual environment of the Nushell process.
pub fn which(name: impl AsRef<OsStr>, paths: &str, cwd: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let paths = format!("{};{}", cwd.display(), paths);
    which::which_in(name, Some(paths), cwd).ok()
}

/// Returns true if `name` is a (somewhat useful) CMD internal command. The full
/// list can be found at <https://ss64.com/nt/syntax-internal.html>
fn is_cmd_internal_command(name: &str) -> bool {
    const COMMANDS: &[&str] = &[
        "ASSOC", "CLS", "ECHO", "FTYPE", "MKLINK", "PAUSE", "START", "VER", "VOL",
    ];
    COMMANDS.iter().any(|cmd| cmd.eq_ignore_ascii_case(name))
}

/// Returns true if a string contains CMD special characters.
fn has_cmd_special_character(s: impl AsRef<[u8]>) -> bool {
    s.as_ref()
        .iter()
        .any(|b| matches!(b, b'<' | b'>' | b'&' | b'|' | b'^'))
}

/// Escape an argument for CMD internal commands. The result can be safely passed to `raw_arg()`.
#[cfg_attr(not(windows), allow(dead_code))]
fn escape_cmd_argument(arg: &Spanned<OsString>) -> Result<Cow<'_, OsStr>, ShellError> {
    let Spanned { item: arg, span } = arg;
    let bytes = arg.as_encoded_bytes();
    if bytes.iter().any(|b| matches!(b, b'\r' | b'\n' | b'%')) {
        // \r and \n truncate the rest of the arguments and % can expand environment variables
        Err(ShellError::ExternalCommand {
            label:
                "Arguments to CMD internal commands cannot contain new lines or percent signs '%'"
                    .into(),
            help: "some characters currently cannot be securely escaped".into(),
            span: *span,
        })
    } else if bytes.contains(&b'"') {
        // If `arg` is already quoted by double quotes, confirm there's no
        // embedded double quotes, then leave it as is.
        if bytes.iter().filter(|b| **b == b'"').count() == 2
            && bytes.starts_with(b"\"")
            && bytes.ends_with(b"\"")
        {
            Ok(Cow::Borrowed(arg))
        } else {
            Err(ShellError::ExternalCommand {
                label: "Arguments to CMD internal commands cannot contain embedded double quotes"
                    .into(),
                help: "this case currently cannot be securely handled".into(),
                span: *span,
            })
        }
    } else if bytes.contains(&b' ') || has_cmd_special_character(bytes) {
        // If `arg` contains space or special characters, quote the entire argument by double quotes.
        let mut new_str = OsString::new();
        new_str.push("\"");
        new_str.push(arg);
        new_str.push("\"");
        Ok(Cow::Owned(new_str))
    } else {
        // FIXME?: what if `arg.is_empty()`?
        Ok(Cow::Borrowed(arg))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_test_support::{fs::Stub, playground::Playground};

    #[test]
    fn test_expand_glob() {
        Playground::setup("test_expand_glob", |dirs, play| {
            play.with_files(&[Stub::EmptyFile("a.txt"), Stub::EmptyFile("b.txt")]);

            let cwd = dirs.test().as_std_path();

            let actual = expand_glob("*.txt", cwd, Span::unknown(), Signals::empty()).unwrap();
            let expected = &["a.txt", "b.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("./*.txt", cwd, Span::unknown(), Signals::empty()).unwrap();
            assert_eq!(actual, expected);

            let actual = expand_glob("'*.txt'", cwd, Span::unknown(), Signals::empty()).unwrap();
            let expected = &["'*.txt'"];
            assert_eq!(actual, expected);

            let actual = expand_glob(".", cwd, Span::unknown(), Signals::empty()).unwrap();
            let expected = &["."];
            assert_eq!(actual, expected);

            let actual = expand_glob("./a.txt", cwd, Span::unknown(), Signals::empty()).unwrap();
            let expected = &["./a.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("[*.txt", cwd, Span::unknown(), Signals::empty()).unwrap();
            let expected = &["[*.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("~/foo.txt", cwd, Span::unknown(), Signals::empty()).unwrap();
            let home = dirs::home_dir().expect("failed to get home dir");
            let expected: Vec<OsString> = vec![home.join("foo.txt").into()];
            assert_eq!(actual, expected);
        })
    }

    #[test]
    fn test_write_pipeline_data() {
        let mut engine_state = EngineState::new();
        let stack = Stack::new();
        let cwd = std::env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();

        // set the PWD environment variable as it's required now
        engine_state.add_env_var("PWD".into(), Value::string(cwd, Span::test_data()));

        let mut buf = vec![];
        let input = PipelineData::empty();
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"");

        let mut buf = vec![];
        let input = PipelineData::value(Value::string("foo", Span::unknown()), None);
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");

        let mut buf = vec![];
        let input = PipelineData::value(Value::binary(b"foo", Span::unknown()), None);
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");

        let mut buf = vec![];
        let input = PipelineData::byte_stream(
            ByteStream::read(
                b"foo".as_slice(),
                Span::unknown(),
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        );
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");
    }
}
