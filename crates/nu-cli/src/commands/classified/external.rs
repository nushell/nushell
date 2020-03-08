use crate::futures::ThreadedReceiver;
use crate::prelude::*;
use bytes::{BufMut, Bytes, BytesMut};
use futures::executor::block_on_stream;
use futures::stream::StreamExt;
use futures_codec::FramedRead;
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
use std::sync::mpsc;

pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}
pub struct MaybeTextCodec;

impl futures_codec::Encoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = std::io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            StringOrBinary::String(s) => {
                dst.reserve(s.len());
                dst.put(s.as_bytes());
                Ok(())
            }
            StringOrBinary::Binary(b) => {
                dst.reserve(b.len());
                dst.put(Bytes::from(b));
                Ok(())
            }
        }
    }
}

impl futures_codec::Decoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let v: Vec<u8> = src.to_vec();
        match String::from_utf8(v) {
            Ok(s) => {
                src.clear();
                if s.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(StringOrBinary::String(s)))
                }
            }
            Err(err) => {
                // Note: the longest UTF-8 character per Unicode spec is currently 6 bytes. If we fail somewhere earlier than the last 6 bytes,
                // we know that we're failing to understand the string encoding and not just seeing a partial character. When this happens, let's
                // fall back to assuming it's a binary buffer.
                if src.is_empty() {
                    Ok(None)
                } else if src.len() > 6 && (src.len() - err.utf8_error().valid_up_to() > 6) {
                    // Fall back to assuming binary
                    let buf = src.to_vec();
                    src.clear();
                    Ok(Some(StringOrBinary::Binary(buf)))
                } else {
                    // Looks like a utf-8 string, so let's assume that
                    let buf = src.split_to(err.utf8_error().valid_up_to() + 1);
                    String::from_utf8(buf.to_vec())
                        .map(|x| Some(StringOrBinary::String(x)))
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }
            }
        }
    }
}

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

fn spawn(
    command: &ExternalCommand,
    path: &str,
    args: &[String],
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let command = command.clone();

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

    // TODO Switch to async_std::process once it's stabilized
    if let Ok(mut child) = process.spawn() {
        let (tx, rx) = mpsc::sync_channel(0);

        let mut stdin = child.stdin.take();

        let stdin_write_tx = tx.clone();
        let stdout_read_tx = tx;
        let stdin_name_tag = command.name_tag.clone();
        let stdout_name_tag = command.name_tag;

        std::thread::spawn(move || {
            if let Some(input) = input {
                let mut stdin_write = stdin
                    .take()
                    .expect("Internal error: could not get stdin pipe for external command");

                for value in block_on_stream(input) {
                    match &value.value {
                        UntaggedValue::Primitive(Primitive::Nothing) => continue,
                        UntaggedValue::Primitive(Primitive::String(s))
                        | UntaggedValue::Primitive(Primitive::Line(s)) => {
                            if let Err(e) = stdin_write.write(s.as_bytes()) {
                                let message = format!("Unable to write to stdin (error = {})", e);

                                let _ = stdin_write_tx.send(Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "application may have closed before completing pipeline",
                                        &stdin_name_tag,
                                    )),
                                    tag: stdin_name_tag,
                                }));
                                return Err(());
                            }
                        }
                        UntaggedValue::Primitive(Primitive::Binary(b)) => {
                            if let Err(e) = stdin_write.write(b) {
                                let message = format!("Unable to write to stdin (error = {})", e);

                                let _ = stdin_write_tx.send(Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "application may have closed before completing pipeline",
                                        &stdin_name_tag,
                                    )),
                                    tag: stdin_name_tag,
                                }));
                                return Err(());
                            }
                        }
                        unsupported => {
                            let _ = stdin_write_tx.send(Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    format!(
                                        "Received unexpected type from pipeline ({})",
                                        unsupported.type_name()
                                    ),
                                    "expected a string",
                                    stdin_name_tag.clone(),
                                )),
                                tag: stdin_name_tag,
                            }));
                            return Err(());
                        }
                    };
                }
            }

            Ok(())
        });

        std::thread::spawn(move || {
            if !is_last {
                let stdout = if let Some(stdout) = child.stdout.take() {
                    stdout
                } else {
                    let _ = stdout_read_tx.send(Ok(Value {
                        value: UntaggedValue::Error(ShellError::labeled_error(
                            "Can't redirect the stdout for external command",
                            "can't redirect stdout",
                            &stdout_name_tag,
                        )),
                        tag: stdout_name_tag,
                    }));
                    return Err(());
                };

                let file = futures::io::AllowStdIo::new(stdout);
                let stream = FramedRead::new(file, MaybeTextCodec);

                for line in block_on_stream(stream) {
                    match line {
                        Ok(line) => match line {
                            StringOrBinary::String(s) => {
                                let result = stdout_read_tx.send(Ok(Value {
                                    value: UntaggedValue::Primitive(Primitive::String(s.clone())),
                                    tag: stdout_name_tag.clone(),
                                }));

                                if result.is_err() {
                                    break;
                                }
                            }
                            StringOrBinary::Binary(b) => {
                                let result = stdout_read_tx.send(Ok(Value {
                                    value: UntaggedValue::Primitive(Primitive::Binary(
                                        b.into_iter().collect(),
                                    )),
                                    tag: stdout_name_tag.clone(),
                                }));

                                if result.is_err() {
                                    break;
                                }
                            }
                        },
                        Err(_) => {
                            let _ = stdout_read_tx.send(Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    "Unable to read from stdout.",
                                    "unable to read from stdout",
                                    &stdout_name_tag,
                                )),
                                tag: stdout_name_tag.clone(),
                            }));
                            break;
                        }
                    }
                }
            }

            // We can give an error when we see a non-zero exit code, but this is different
            // than what other shells will do.
            if child.wait().is_err() {
                let cfg = crate::data::config::config(Tag::unknown());
                if let Ok(cfg) = cfg {
                    if cfg.contains_key("nonzero_exit_errors") {
                        let _ = stdout_read_tx.send(Ok(Value {
                            value: UntaggedValue::Error(ShellError::labeled_error(
                                "External command failed",
                                "command failed",
                                &stdout_name_tag,
                            )),
                            tag: stdout_name_tag,
                        }));
                    }
                }
            }

            Ok(())
        });

        let stream = ThreadedReceiver::new(rx);
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

    (argument.starts_with('"') && argument.ends_with('"'))
        || (argument.starts_with('\'') && argument.ends_with('\''))
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
