use nu_cmd_base::hook::eval_hook;
use nu_engine::{command_prelude::*, env_to_strings, get_eval_expression};
use nu_path::{dots::expand_ndots, expand_tilde};
use nu_protocol::{
    ast::Expression, did_you_mean, process::ChildProcess, ByteStream, NuGlob, OutDest,
};
use nu_system::ForegroundChild;
use nu_utils::IgnoreCaseExt;
use pathdiff::diff_paths;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{atomic::AtomicBool, Arc},
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
            .required(
                "command",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
                "External command to run.",
            )
            .rest(
                "args",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Any]),
                "Arguments for external command.",
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

        let name: Value = call.req(engine_state, stack, 0)?;

        let name_str: Cow<str> = match &name {
            Value::Glob { val, .. } => Cow::Borrowed(val),
            Value::String { val, .. } => Cow::Borrowed(val),
            _ => Cow::Owned(name.clone().coerce_into_string()?),
        };

        let expanded_name = match &name {
            // Expand tilde and ndots on the name if it's a bare string / glob (#13000)
            Value::Glob { no_expand, .. } if !*no_expand => {
                expand_ndots_safe(expand_tilde(&*name_str))
            }
            _ => Path::new(&*name_str).to_owned(),
        };

        // Find the absolute path to the executable. On Windows, set the
        // executable to "cmd.exe" if it's is a CMD internal command. If the
        // command is not found, display a helpful error message.
        let executable = if cfg!(windows) && is_cmd_internal_command(&name_str) {
            PathBuf::from("cmd.exe")
        } else {
            // Determine the PATH to be used and then use `which` to find it - though this has no
            // effect if it's an absolute path already
            let paths = nu_engine::env::path_str(engine_state, stack, call.head)?;
            let Some(executable) = which(expanded_name, &paths, &cwd) else {
                return Err(command_not_found(&name_str, call.head, engine_state, stack));
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
        #[cfg(windows)]
        if is_cmd_internal_command(&name_str) {
            use std::os::windows::process::CommandExt;

            // The /D flag disables execution of AutoRun commands from registry.
            // The /C flag followed by a command name instructs CMD to execute
            // that command and quit.
            command.args(["/D", "/C", &name_str]);
            for arg in &args {
                command.raw_arg(escape_cmd_argument(arg)?);
            }
        } else {
            command.args(args.into_iter().map(|s| s.item));
        }
        #[cfg(not(windows))]
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

        // Log the command we're about to run in case it's useful for debugging purposes.
        log::trace!("run-external spawning: {command:?}");

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
            let engine_state = engine_state.clone();
            let stack = stack.clone();
            thread::Builder::new()
                .name("external stdin worker".into())
                .spawn(move || {
                    let _ = write_pipeline_data(engine_state, stack, data, stdin);
                })
                .err_span(call.head)?;
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

/// Evaluate all arguments from a call, performing expansions when necessary.
pub fn eval_arguments_from_call(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Vec<Spanned<OsString>>, ShellError> {
    let ctrlc = &engine_state.ctrlc;
    let cwd = engine_state.cwd(Some(stack))?;
    let mut args: Vec<Spanned<OsString>> = vec![];
    for (expr, spread) in call.rest_iter(1) {
        for arg in eval_argument(engine_state, stack, expr, spread)? {
            match arg {
                // Expand globs passed to run-external
                Value::Glob { val, no_expand, .. } if !no_expand => args.extend(
                    expand_glob(&val, &cwd, expr.span, ctrlc)?
                        .into_iter()
                        .map(|s| s.into_spanned(expr.span)),
                ),
                other => {
                    args.push(OsString::from(coerce_into_string(other)?).into_spanned(expr.span))
                }
            }
        }
    }
    Ok(args)
}

/// Custom `coerce_into_string()`, including globs, since those are often args to `run-external`
/// as well
fn coerce_into_string(val: Value) -> Result<String, ShellError> {
    match val {
        Value::Glob { val, .. } => Ok(val),
        _ => val.coerce_into_string(),
    }
}

/// Evaluate an argument, returning more than one value if it was a list to be spread.
fn eval_argument(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    spread: bool,
) -> Result<Vec<Value>, ShellError> {
    let eval = get_eval_expression(engine_state);
    match eval(engine_state, stack, expr)? {
        Value::List { vals, .. } => {
            if spread {
                Ok(vals)
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
                Ok(vec![value])
            }
        }
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
    interrupt: &Option<Arc<AtomicBool>>,
) -> Result<Vec<OsString>, ShellError> {
    const GLOB_CHARS: &[char] = &['*', '?', '['];

    // For an argument that doesn't include the GLOB_CHARS, just do the `expand_tilde`
    // and `expand_ndots` expansion
    if !arg.contains(GLOB_CHARS) {
        let path = expand_ndots_safe(expand_tilde(arg));
        return Ok(vec![path.into()]);
    }

    // We must use `nu_engine::glob_from` here, in order to ensure we get paths from the correct
    // dir
    let glob = NuGlob::Expand(arg.to_owned()).into_spanned(span);
    if let Ok((prefix, matches)) = nu_engine::glob_from(&glob, cwd, span, None) {
        let mut result: Vec<OsString> = vec![];

        for m in matches {
            if nu_utils::ctrl_c::was_pressed(interrupt) {
                return Err(ShellError::InterruptedByUser { span: Some(span) });
            }
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
/// extract the pipe from the `PipelineData::ByteStream` of the first command
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
        writer.write_all(&val)?;
    } else {
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
/// list can be found at https://ss64.com/nt/syntax-internal.html
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
        // \r and \n trunacte the rest of the arguments and % can expand environment variables
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

/// Expand ndots, but only if it looks like it probably contains them, because there is some lossy
/// path normalization that happens.
fn expand_ndots_safe(path: impl AsRef<Path>) -> PathBuf {
    let string = path.as_ref().to_string_lossy();

    // Use ndots if it contains at least `...`, since that's the minimum trigger point, and don't
    // use it if it contains ://, because that looks like a URL scheme and the path normalization
    // will mess with that.
    if string.contains("...") && !string.contains("://") {
        expand_ndots(path)
    } else {
        path.as_ref().to_owned()
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

            let cwd = dirs.test();

            let actual = expand_glob("*.txt", cwd, Span::unknown(), &None).unwrap();
            let expected = &["a.txt", "b.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("./*.txt", cwd, Span::unknown(), &None).unwrap();
            assert_eq!(actual, expected);

            let actual = expand_glob("'*.txt'", cwd, Span::unknown(), &None).unwrap();
            let expected = &["'*.txt'"];
            assert_eq!(actual, expected);

            let actual = expand_glob(".", cwd, Span::unknown(), &None).unwrap();
            let expected = &["."];
            assert_eq!(actual, expected);

            let actual = expand_glob("./a.txt", cwd, Span::unknown(), &None).unwrap();
            let expected = &["./a.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("[*.txt", cwd, Span::unknown(), &None).unwrap();
            let expected = &["[*.txt"];
            assert_eq!(actual, expected);

            let actual = expand_glob("~/foo.txt", cwd, Span::unknown(), &None).unwrap();
            let home = dirs_next::home_dir().expect("failed to get home dir");
            let expected: Vec<OsString> = vec![home.join("foo.txt").into()];
            assert_eq!(actual, expected);
        })
    }

    #[test]
    fn test_write_pipeline_data() {
        let engine_state = EngineState::new();
        let stack = Stack::new();

        let mut buf = vec![];
        let input = PipelineData::Empty;
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"");

        let mut buf = vec![];
        let input = PipelineData::Value(Value::string("foo", Span::unknown()), None);
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");

        let mut buf = vec![];
        let input = PipelineData::Value(Value::binary(b"foo", Span::unknown()), None);
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
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
        write_pipeline_data(engine_state.clone(), stack.clone(), input, &mut buf).unwrap();
        assert_eq!(buf, b"foo");
    }
}
