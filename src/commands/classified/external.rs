use crate::futures::ThreadedFuture;
use crate::prelude::*;
use futures::stream::StreamExt;
use futures_codec::{FramedRead, LinesCodec};
use log::trace;
use nu_errors::ShellError;
use nu_parser::commands::classified::external::ExternalArg;
use nu_parser::ExternalCommand;
use nu_protocol::{ColumnPath, Primitive, ShellTypeName, UntaggedValue, Value};
use nu_source::{Tag, Tagged};
use nu_value_ext::as_column_path;
use std::io::Write;
use std::ops::Deref;
use std::process::{Command, Stdio};

pub fn nu_value_to_string(command: &ExternalCommand, from: &Value) -> Result<String, ShellError> {
    match &from.value {
        UntaggedValue::Primitive(Primitive::Int(i)) => Ok(i.to_string()),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => Ok(s.clone()),
        UntaggedValue::Primitive(Primitive::Path(p)) => Ok(p.to_string_lossy().to_string()),
        unsupported => Err(ShellError::labeled_error(
            format!("needs string data (given: {})", unsupported.type_name()),
            "expected a string",
            &command.name_tag,
        )),
    }
}

pub fn nu_value_to_string_for_stdin(
    command: &ExternalCommand,
    from: &Value,
) -> Result<Option<String>, ShellError> {
    match &from.value {
        UntaggedValue::Primitive(Primitive::Nothing) => Ok(None),
        UntaggedValue::Primitive(Primitive::String(s))
        | UntaggedValue::Primitive(Primitive::Line(s)) => Ok(Some(s.clone())),
        unsupported => Err(ShellError::labeled_error(
            format!(
                "Received unexpected type from pipeline ({})",
                unsupported.type_name()
            ),
            "expected a string",
            &command.name_tag,
        )),
    }
}

pub(crate) fn run_external_command(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    trace!(target: "nu::run::external", "-> {}", command.name);

    if !did_find_command(&command.name) {
        return Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            &command.name_tag,
        ));
    }

    if command.has_it_argument() || command.has_nu_argument() {
        run_with_iterator_arg(command, context, input, is_last)
    } else {
        run_with_stdin(command, context, input, is_last)
    }
}

fn prepare_column_path_for_fetching_it_variable(
    argument: &ExternalArg,
) -> Result<Tagged<ColumnPath>, ShellError> {
    // We have "$it.[contents of interest]"
    // and start slicing from "$it.[member+]"
    //                             ^ here.
    let key = nu_source::Text::from(argument.deref()).slice(4..argument.len());

    to_column_path(&key, &argument.tag)
}

fn prepare_column_path_for_fetching_nu_variable(
    argument: &ExternalArg,
) -> Result<Tagged<ColumnPath>, ShellError> {
    // We have "$nu.[contents of interest]"
    // and start slicing from "$nu.[member+]"
    //                             ^ here.
    let key = nu_source::Text::from(argument.deref()).slice(4..argument.len());

    to_column_path(&key, &argument.tag)
}

fn to_column_path(
    path_members: &str,
    tag: impl Into<Tag>,
) -> Result<Tagged<ColumnPath>, ShellError> {
    let tag = tag.into();

    as_column_path(
        &UntaggedValue::Table(
            path_members
                .split('.')
                .map(|x| {
                    let member = match x.parse::<u64>() {
                        Ok(v) => UntaggedValue::int(v),
                        Err(_) => UntaggedValue::string(x),
                    };

                    member.into_value(&tag)
                })
                .collect(),
        )
        .into_value(&tag),
    )
}

fn run_with_iterator_arg(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let path = context.shell_manager.path();

    let mut inputs: InputStream = if let Some(input) = input {
        trace_stream!(target: "nu::trace_stream::external::it", "input" = input)
    } else {
        InputStream::empty()
    };

    let stream = async_stream! {
        while let Some(value) = inputs.next().await {
            let name = command.name.clone();
            let name_tag = command.name_tag.clone();
            let home_dir = dirs::home_dir();
            let path = &path;
            let args = command.args.clone();

            let it_replacement = {
                if command.has_it_argument() {
                    let empty_arg = ExternalArg {
                        arg: "".to_string(),
                        tag: name_tag.clone()
                    };

                    let key = args.iter()
                        .find(|arg| arg.looks_like_it())
                        .unwrap_or_else(|| &empty_arg);

                    if args.iter().all(|arg| !arg.is_it()) {
                        let key = match prepare_column_path_for_fetching_it_variable(&key) {
                            Ok(keypath) => keypath,
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            }
                        };

                        match crate::commands::get::get_column_path(&key, &value) {
                            Ok(field) => {
                                match nu_value_to_string(&command, &field) {
                                    Ok(val) => Some(val),
                                    Err(reason) => {
                                        yield Ok(Value {
                                            value: UntaggedValue::Error(reason),
                                            tag: name_tag
                                        });
                                        return;
                                    },
                                }
                            },
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            }
                        }
                    } else {
                        match nu_value_to_string(&command, &value) {
                            Ok(val) => Some(val),
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            },
                        }
                    }
                } else {
                    None
                }
            };

            let nu_replacement = {
                if command.has_nu_argument() {
                    let empty_arg = ExternalArg {
                        arg: "".to_string(),
                        tag: name_tag.clone()
                    };

                    let key = args.iter()
                        .find(|arg| arg.looks_like_nu())
                        .unwrap_or_else(|| &empty_arg);

                    let nu_var = match crate::evaluate::variables::nu(&name_tag) {
                        Ok(variables) => variables,
                        Err(reason) => {
                            yield Ok(Value {
                                value: UntaggedValue::Error(reason),
                                tag: name_tag
                            });
                            return;
                        }
                    };

                    if args.iter().all(|arg| !arg.is_nu()) {
                        let key = match prepare_column_path_for_fetching_nu_variable(&key) {
                            Ok(keypath) => keypath,
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            }
                        };

                        match crate::commands::get::get_column_path(&key, &nu_var) {
                            Ok(field) => {
                                match nu_value_to_string(&command, &field) {
                                    Ok(val) => Some(val),
                                    Err(reason) => {
                                        yield Ok(Value {
                                            value: UntaggedValue::Error(reason),
                                            tag: name_tag
                                        });
                                        return;
                                    },
                                }
                            },
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            }
                        }
                    } else {
                        match nu_value_to_string(&command, &nu_var) {
                            Ok(val) => Some(val),
                            Err(reason) => {
                                yield Ok(Value {
                                    value: UntaggedValue::Error(reason),
                                    tag: name_tag
                                });
                                return;
                            },
                        }
                    }
                } else {
                    None
                }
            };

            let process_args = args.iter().filter_map(|arg| {
                if arg.chars().all(|c| c.is_whitespace()) {
                    None
                } else {
                    let arg = if arg.looks_like_it() {
                        if let Some(mut value) = it_replacement.to_owned() {
                            let mut value = expand_tilde(&value, || home_dir.as_ref()).as_ref().to_string();
                            #[cfg(not(windows))]
                            {
                                value = {
                                    if argument_contains_whitespace(&value) && !argument_is_quoted(&value) {
                                        add_quotes(&value)
                                    } else {
                                        value
                                    }
                                };
                            }
                            Some(value)
                        } else {
                            None
                        }
                    } else if arg.looks_like_nu() {
                        if let Some(mut value) = nu_replacement.to_owned() {
                            #[cfg(not(windows))]
                            {
                                value = {
                                    if argument_contains_whitespace(&value) && !argument_is_quoted(&value) {
                                        add_quotes(&value)
                                    } else {
                                        value
                                    }
                                };
                            }
                            Some(value)
                        } else {
                            None
                        }
                    } else {
                        Some(arg.to_string())
                    };

                    arg
                }
            }).collect::<Vec<String>>();

            match spawn(&command, &path, &process_args[..], None, is_last) {
                Ok(res) => {
                    if let Some(mut res) = res {
                        while let Some(item) = res.next().await {
                            yield Ok(item)
                        }
                    }
                }
                Err(reason) => {
                    yield Ok(Value {
                        value: UntaggedValue::Error(reason),
                        tag: name_tag
                    });
                    return;
                }
            }
        }
    };

    Ok(Some(stream.to_input_stream()))
}

fn run_with_stdin(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let path = context.shell_manager.path();

    let input = input
        .map(|input| trace_stream!(target: "nu::trace_stream::external::stdin", "input" = input));

    let process_args = command
        .args
        .iter()
        .map(|arg| {
            let arg = expand_tilde(arg.deref(), dirs::home_dir);

            #[cfg(not(windows))]
            {
                if argument_contains_whitespace(&arg) && argument_is_quoted(&arg) {
                    if let Some(unquoted) = remove_quotes(&arg) {
                        format!(r#""{}""#, unquoted)
                    } else {
                        arg.as_ref().to_string()
                    }
                } else {
                    arg.as_ref().to_string()
                }
            }
            #[cfg(windows)]
            {
                if let Some(unquoted) = remove_quotes(&arg) {
                    unquoted.to_string()
                } else {
                    arg.as_ref().to_string()
                }
            }
        })
        .collect::<Vec<String>>();

    spawn(&command, &path, &process_args[..], input, is_last)
}

/// This is a wrapper for stdout-like readers that ensure a carriage return ends the stream
pub struct StdoutWithNewline<T: std::io::Read> {
    stdout: T,
    ended_in_newline: bool,
}

impl<T: std::io::Read> StdoutWithNewline<T> {
    pub fn new(stdout: T) -> StdoutWithNewline<T> {
        StdoutWithNewline {
            stdout,
            ended_in_newline: false,
        }
    }
}
impl<T: std::io::Read> std::io::Read for StdoutWithNewline<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.stdout.read(buf) {
            Err(e) => Err(e),
            Ok(0) => {
                if !self.ended_in_newline && !buf.is_empty() {
                    self.ended_in_newline = true;
                    buf[0] = b'\n';
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Ok(len) => {
                if buf[len - 1] == b'\n' {
                    self.ended_in_newline = true;
                } else {
                    self.ended_in_newline = false;
                }
                Ok(len)
            }
        }
    }
}

fn spawn(
    command: &ExternalCommand,
    path: &str,
    args: &[String],
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let command = command.clone();
    let name_tag = command.name_tag.clone();

    let mut process = {
        #[cfg(windows)]
        {
            let mut process = Command::new("cmd");
            process.arg("/c");
            process.arg(&command.name);
            for arg in args {
                process.arg(&arg);
            }
            process
        }

        #[cfg(not(windows))]
        {
            let cmd_with_args = vec![command.name.clone(), args.join(" ")].join(" ");
            let mut process = Command::new("sh");
            process.arg("-c").arg(cmd_with_args);
            process
        }
    };

    process.current_dir(path);
    trace!(target: "nu::run::external", "cwd = {:?}", &path);

    // We want stdout regardless of what
    // we are doing ($it case or pipe stdin)
    if !is_last {
        process.stdout(Stdio::piped());
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    // open since we have some contents for stdin
    if input.is_some() {
        process.stdin(Stdio::piped());
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built command {:?}", process);

    if let Ok(mut child) = process.spawn() {
        let stream = async_stream! {
            if let Some(mut input) = input {
                let mut stdin_write = child.stdin
                    .take()
                    .expect("Internal error: could not get stdin pipe for external command");

                while let Some(value) = input.next().await {
                    let input_string = match nu_value_to_string_for_stdin(&command, &value) {
                        Ok(None) => continue,
                        Ok(Some(v)) => v,
                        Err(e) => {
                            yield Ok(Value {
                                value: UntaggedValue::Error(e),
                                tag: name_tag
                            });
                            return;
                        }
                    };

                    if let Err(e) = stdin_write.write(input_string.as_bytes()) {
                        let message = format!("Unable to write to stdin (error = {})", e);

                        yield Ok(Value {
                            value: UntaggedValue::Error(ShellError::labeled_error(
                                message,
                                "application may have closed before completing pipeline",
                                &name_tag)),
                            tag: name_tag
                        });
                        return;
                    }
                }
            }

            if !is_last {
                let stdout = if let Some(stdout) = child.stdout.take() {
                    stdout
                } else {
                    yield Ok(Value {
                        value: UntaggedValue::Error(ShellError::labeled_error(
                            "Can't redirect the stdout for external command",
                            "can't redirect stdout",
                            &name_tag)),
                        tag: name_tag
                    });
                    return;
                };

                let file = futures::io::AllowStdIo::new(StdoutWithNewline::new(stdout));
                let mut stream = FramedRead::new(file, LinesCodec);

                while let Some(line) = stream.next().await {
                    if let Ok(line) = line {
                        yield Ok(Value {
                            value: UntaggedValue::Primitive(Primitive::Line(line)),
                            tag: name_tag.clone(),
                        });
                    } else {
                        yield Ok(Value {
                            value: UntaggedValue::Error(
                                ShellError::labeled_error(
                                    "Unable to read lines from stdout. This usually happens when the output does not end with a newline.",
                                    "unable to read from stdout",
                                    &name_tag,
                                )
                            ),
                            tag: name_tag.clone(),
                        });
                        return;
                    }
                }
            }

            // TODO Switch to async_std::process once it's stabilized
            let result = ThreadedFuture::new(move || {
                // We can give an error when we see a non-zero exit code, but this is different
                // than what other shells will do.
                if child.wait().is_err() {
                    let cfg = crate::data::config::config(Tag::unknown());
                    if let Ok(cfg) = cfg {
                        if cfg.contains_key("nonzero_exit_errors") {
                            return Some(Value {
                                value: UntaggedValue::Error(
                                    ShellError::labeled_error(
                                        "External command failed",
                                        "command failed",
                                        &name_tag,
                                    )
                                ),
                                tag: name_tag,
                            });
                        }
                    }
                }
                None
            }).await;

            if let Some(v) = result {
                yield Ok(v);
            }
        };

        Ok(Some(stream.to_input_stream()))
    } else {
        Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            &command.name_tag,
        ))
    }
}

fn did_find_command(name: &str) -> bool {
    #[cfg(not(windows))]
    {
        which::which(name).is_ok()
    }

    #[cfg(windows)]
    {
        if which::which(name).is_ok() {
            true
        } else {
            let cmd_builtins = [
                "call", "cls", "color", "date", "dir", "echo", "find", "hostname", "pause",
                "start", "time", "title", "ver", "copy", "mkdir", "rename", "rd", "rmdir", "type",
            ];

            cmd_builtins.contains(&name)
        }
    }
}

fn expand_tilde<SI: ?Sized, P, HD>(input: &SI, home_dir: HD) -> std::borrow::Cow<str>
where
    SI: AsRef<str>,
    P: AsRef<std::path::Path>,
    HD: FnOnce() -> Option<P>,
{
    shellexpand::tilde_with_context(input, home_dir)
}

#[allow(unused)]
pub fn argument_contains_whitespace(argument: &str) -> bool {
    argument.chars().any(|c| c.is_whitespace())
}

fn argument_is_quoted(argument: &str) -> bool {
    if argument.len() < 2 {
        return false;
    }

    ((argument.starts_with('"') && argument.ends_with('"'))
        || (argument.starts_with('\'') && argument.ends_with('\'')))
}

#[allow(unused)]
fn add_quotes(argument: &str) -> String {
    format!("\"{}\"", argument)
}

fn remove_quotes(argument: &str) -> Option<&str> {
    if !argument_is_quoted(argument) {
        return None;
    }

    let size = argument.len();

    Some(&argument[1..size - 1])
}

#[allow(unused)]
fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os("PATH") {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

#[cfg(test)]
mod tests {
    use super::{
        add_quotes, argument_contains_whitespace, argument_is_quoted, expand_tilde, remove_quotes,
        run_external_command, Context,
    };
    use futures::executor::block_on;
    use nu_errors::ShellError;
    use nu_test_support::commands::ExternalBuilder;

    // async fn read(mut stream: OutputStream) -> Option<Value> {
    //     match stream.try_next().await {
    //         Ok(val) => {
    //             if let Some(val) = val {
    //                 val.raw_value()
    //             } else {
    //                 None
    //             }
    //         }
    //         Err(_) => None,
    //     }
    // }

    async fn non_existent_run() -> Result<(), ShellError> {
        let cmd = ExternalBuilder::for_name("i_dont_exist.exe").build();

        let mut ctx = Context::basic().expect("There was a problem creating a basic context.");

        assert!(run_external_command(cmd, &mut ctx, None, false).is_err());

        Ok(())
    }

    // async fn failure_run() -> Result<(), ShellError> {
    //     let cmd = ExternalBuilder::for_name("fail").build();

    //     let mut ctx = Context::basic().expect("There was a problem creating a basic context.");
    //     let stream = run_external_command(cmd, &mut ctx, None, false)
    //         .await?
    //         .expect("There was a problem running the external command.");

    //     match read(stream.into()).await {
    //         Some(Value {
    //             value: UntaggedValue::Error(_),
    //             ..
    //         }) => {}
    //         None | _ => panic!("Command didn't fail."),
    //     }

    //     Ok(())
    // }

    // #[test]
    // fn identifies_command_failed() -> Result<(), ShellError> {
    //     block_on(failure_run())
    // }

    #[test]
    fn identifies_command_not_found() -> Result<(), ShellError> {
        block_on(non_existent_run())
    }

    #[test]
    fn checks_contains_whitespace_from_argument_to_be_passed_in() {
        assert_eq!(argument_contains_whitespace("andrés"), false);
        assert_eq!(argument_contains_whitespace("and rés"), true);
        assert_eq!(argument_contains_whitespace(r#"and\ rés"#), true);
    }

    #[test]
    fn checks_quotes_from_argument_to_be_passed_in() {
        assert_eq!(argument_is_quoted(""), false);

        assert_eq!(argument_is_quoted("'"), false);
        assert_eq!(argument_is_quoted("'a"), false);
        assert_eq!(argument_is_quoted("a"), false);
        assert_eq!(argument_is_quoted("a'"), false);
        assert_eq!(argument_is_quoted("''"), true);

        assert_eq!(argument_is_quoted(r#"""#), false);
        assert_eq!(argument_is_quoted(r#""a"#), false);
        assert_eq!(argument_is_quoted(r#"a"#), false);
        assert_eq!(argument_is_quoted(r#"a""#), false);
        assert_eq!(argument_is_quoted(r#""""#), true);

        assert_eq!(argument_is_quoted("'andrés"), false);
        assert_eq!(argument_is_quoted("andrés'"), false);
        assert_eq!(argument_is_quoted(r#""andrés"#), false);
        assert_eq!(argument_is_quoted(r#"andrés""#), false);
        assert_eq!(argument_is_quoted("'andrés'"), true);
        assert_eq!(argument_is_quoted(r#""andrés""#), true);
    }

    #[test]
    fn adds_quotes_to_argument_to_be_passed_in() {
        assert_eq!(add_quotes("andrés"), "\"andrés\"");
        //assert_eq!(add_quotes("\"andrés\""), "\"andrés\"");
    }

    #[test]
    fn strips_quotes_from_argument_to_be_passed_in() {
        assert_eq!(remove_quotes(""), None);

        assert_eq!(remove_quotes("'"), None);
        assert_eq!(remove_quotes("'a"), None);
        assert_eq!(remove_quotes("a"), None);
        assert_eq!(remove_quotes("a'"), None);
        assert_eq!(remove_quotes("''"), Some(""));

        assert_eq!(remove_quotes(r#"""#), None);
        assert_eq!(remove_quotes(r#""a"#), None);
        assert_eq!(remove_quotes(r#"a"#), None);
        assert_eq!(remove_quotes(r#"a""#), None);
        assert_eq!(remove_quotes(r#""""#), Some(""));

        assert_eq!(remove_quotes("'andrés"), None);
        assert_eq!(remove_quotes("andrés'"), None);
        assert_eq!(remove_quotes(r#""andrés"#), None);
        assert_eq!(remove_quotes(r#"andrés""#), None);
        assert_eq!(remove_quotes("'andrés'"), Some("andrés"));
        assert_eq!(remove_quotes(r#""andrés""#), Some("andrés"));
    }

    #[test]
    fn expands_tilde_if_starts_with_tilde_character() {
        assert_eq!(
            expand_tilde("~", || Some(std::path::Path::new("the_path_to_nu_light"))),
            "the_path_to_nu_light"
        );
    }

    #[test]
    fn does_not_expand_tilde_if_tilde_is_not_first_character() {
        assert_eq!(
            expand_tilde("1~1", || Some(std::path::Path::new("the_path_to_nu_light"))),
            "1~1"
        );
    }
}
