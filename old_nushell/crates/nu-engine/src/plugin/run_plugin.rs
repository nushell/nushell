use crate::{command_args::CommandArgs, evaluate_baseline_expr, UnevaluatedCallInfo};
use crate::{
    whole_stream_command::{whole_stream_command, WholeStreamCommand},
    EvaluationContext,
};
use derive_new::new;

use indexmap::IndexMap;
use log::trace;
use nu_errors::ShellError;
use nu_plugin::jsonrpc::JsonRpc;
use nu_protocol::{hir, Primitive, ReturnValue, Signature, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::{ActionStream, InputStream, IntoActionStream};
use serde::{self, Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
pub enum NuResult {
    response {
        params: Result<VecDeque<ReturnValue>, ShellError>,
    },
}

enum PluginCommand {
    Filter(PluginFilter),
    Sink(PluginSink),
}

impl PluginCommand {
    fn command(self) -> crate::whole_stream_command::Command {
        match self {
            PluginCommand::Filter(cmd) => whole_stream_command(cmd),
            PluginCommand::Sink(cmd) => whole_stream_command(cmd),
        }
    }
}

enum PluginMode {
    Filter,
    Sink,
}

pub struct PluginCommandBuilder {
    mode: PluginMode,
    name: String,
    path: String,
    config: Signature,
}

impl PluginCommandBuilder {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        config: impl Into<Signature>,
    ) -> Self {
        let config = config.into();

        PluginCommandBuilder {
            mode: if config.is_filter {
                PluginMode::Filter
            } else {
                PluginMode::Sink
            },
            name: name.into(),
            path: path.into(),
            config,
        }
    }

    pub fn build(&self) -> crate::whole_stream_command::Command {
        let mode = &self.mode;

        let name = self.name.clone();
        let path = self.path.clone();
        let config = self.config.clone();

        let cmd = match mode {
            PluginMode::Filter => PluginCommand::Filter(PluginFilter { name, path, config }),
            PluginMode::Sink => PluginCommand::Sink(PluginSink { name, path, config }),
        };

        cmd.command()
    }
}

#[derive(new)]
pub struct PluginFilter {
    name: String,
    path: String,
    config: Signature,
}

impl WholeStreamCommand for PluginFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.config.clone()
    }

    fn usage(&self) -> &str {
        &self.config.usage
    }

    fn extra_usage(&self) -> &str {
        &self.config.extra_usage
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run_filter(self.path.clone(), args)
    }

    fn is_plugin(&self) -> bool {
        true
    }

    fn is_builtin(&self) -> bool {
        false
    }
}

fn run_filter(path: String, args: CommandArgs) -> Result<ActionStream, ShellError> {
    trace!("filter_plugin :: {}", path);

    let bos = vec![UntaggedValue::Primitive(Primitive::BeginningOfStream).into_untagged_value()]
        .into_iter();
    let eos = [UntaggedValue::Primitive(Primitive::EndOfStream).into_untagged_value()];

    let (call_info, input) = evaluate_once(args)?;

    let real_path = Path::new(&path);
    let ext = real_path.extension();
    let ps1_file = match ext {
        Some(ext) => ext == "ps1",
        None => false,
    };

    let mut child: Child = if ps1_file {
        Command::new("pwsh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args([
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &real_path.to_string_lossy(),
            ])
            .spawn()
            .expect("Failed to spawn PowerShell process")
    } else {
        Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process")
    };

    trace!("filtering :: {:?}", call_info);

    Ok(bos
        .chain(input)
        .chain(eos)
        .flat_map(move |item| {
            match item {
                Value {
                    value: UntaggedValue::Primitive(Primitive::BeginningOfStream),
                    ..
                } => {
                    // Beginning of the stream
                    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                    let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                    let mut reader = BufReader::new(stdout);

                    let request = JsonRpc::new("begin_filter", call_info.clone());
                    let request_raw = serde_json::to_string(&request);
                    trace!("begin_filter:request {:?}", &request_raw);

                    match request_raw {
                        Err(_) => {
                            return ActionStream::one(Err(ShellError::labeled_error(
                                "Could not load json from plugin",
                                "could not load json from plugin",
                                &call_info.name_tag,
                            )));
                        }
                        Ok(request_raw) => {
                            match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                                Ok(_) => {}
                                Err(err) => {
                                    return ActionStream::one(Err(ShellError::unexpected(
                                        err.to_string(),
                                    )));
                                }
                            }
                        }
                    }

                    let mut input = String::new();
                    match reader.read_line(&mut input) {
                        Ok(_) => {
                            let response = serde_json::from_str::<NuResult>(&input);
                            trace!("begin_filter:response {:?}", &response);

                            match response {
                                Ok(NuResult::response { params }) => match params {
                                    Ok(params) => params.into_iter().into_action_stream(),
                                    Err(e) => {
                                        vec![ReturnValue::Err(e)].into_iter().into_action_stream()
                                    }
                                },

                                Err(e) => ActionStream::one(Err(
                                    ShellError::untagged_runtime_error(format!(
                                        "Error while processing begin_filter response: {:?} {}",
                                        e, input
                                    )),
                                )),
                            }
                        }
                        Err(e) => ActionStream::one(Err(ShellError::untagged_runtime_error(
                            format!("Error while reading begin_filter response: {:?}", e),
                        ))),
                    }
                }
                Value {
                    value: UntaggedValue::Primitive(Primitive::EndOfStream),
                    ..
                } => {
                    // post stream contents
                    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                    let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                    let mut reader = BufReader::new(stdout);

                    let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("end_filter", vec![]);
                    let request_raw = serde_json::to_string(&request);
                    trace!("end_filter:request {:?}", &request_raw);

                    match request_raw {
                        Err(_) => {
                            return ActionStream::one(Err(ShellError::labeled_error(
                                "Could not load json from plugin",
                                "could not load json from plugin",
                                &call_info.name_tag,
                            )));
                        }
                        Ok(request_raw) => {
                            match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                                Ok(_) => {}
                                Err(err) => {
                                    return ActionStream::one(Err(ShellError::unexpected(
                                        err.to_string(),
                                    )));
                                }
                            }
                        }
                    }

                    let mut input = String::new();
                    let stream = match reader.read_line(&mut input) {
                        Ok(_) => {
                            let response = serde_json::from_str::<NuResult>(&input);
                            trace!("end_filter:response {:?}", &response);

                            match response {
                                Ok(NuResult::response { params }) => match params {
                                    Ok(params) => params.into_iter().into_action_stream(),
                                    Err(e) => {
                                        vec![ReturnValue::Err(e)].into_iter().into_action_stream()
                                    }
                                },
                                Err(e) => vec![Err(ShellError::untagged_runtime_error(format!(
                                    "Error while processing end_filter response: {:?} {}",
                                    e, input
                                )))]
                                .into_iter()
                                .into_action_stream(),
                            }
                        }
                        Err(e) => vec![Err(ShellError::untagged_runtime_error(format!(
                            "Error while reading end_filter response: {:?}",
                            e
                        )))]
                        .into_iter()
                        .into_action_stream(),
                    };

                    let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                    let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("quit", vec![]);
                    let request_raw = serde_json::to_string(&request);
                    trace!("quit:request {:?}", &request_raw);

                    match request_raw {
                        Ok(request_raw) => {
                            let _ = stdin.write(format!("{}\n", request_raw).as_bytes());
                            // TODO: Handle error
                        }
                        Err(e) => {
                            return ActionStream::one(Err(ShellError::untagged_runtime_error(
                                format!("Error while processing quit response: {:?}", e),
                            )));
                        }
                    }
                    let _ = child.wait();

                    stream
                }

                v => {
                    // Stream contents
                    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                    let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                    let mut reader = BufReader::new(stdout);

                    let request = JsonRpc::new("filter", v);
                    let request_raw = serde_json::to_string(&request);
                    trace!("filter:request {:?}", &request_raw);

                    match request_raw {
                        Ok(request_raw) => {
                            let _ = stdin.write(format!("{}\n", request_raw).as_bytes());
                            // TODO: Handle error
                        }
                        Err(e) => {
                            return ActionStream::one(Err(ShellError::untagged_runtime_error(
                                format!("Error while processing filter response: {:?}", e),
                            )));
                        }
                    }

                    let mut input = String::new();
                    match reader.read_line(&mut input) {
                        Ok(_) => {
                            let response = serde_json::from_str::<NuResult>(&input);
                            trace!("filter:response {:?}", &response);

                            match response {
                                Ok(NuResult::response { params }) => match params {
                                    Ok(params) => params.into_iter().into_action_stream(),
                                    Err(e) => {
                                        vec![ReturnValue::Err(e)].into_iter().into_action_stream()
                                    }
                                },
                                Err(e) => ActionStream::one(Err(
                                    ShellError::untagged_runtime_error(format!(
                                    "Error while processing filter response: {:?}\n== input ==\n{}",
                                    e, input
                                )),
                                )),
                            }
                        }
                        Err(e) => ActionStream::one(Err(ShellError::untagged_runtime_error(
                            format!("Error while reading filter response: {:?}", e),
                        ))),
                    }
                }
            }
        })
        .into_action_stream())
}

#[derive(new)]
pub struct PluginSink {
    name: String,
    path: String,
    config: Signature,
}

impl WholeStreamCommand for PluginSink {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.config.clone()
    }

    fn usage(&self) -> &str {
        &self.config.usage
    }

    fn extra_usage(&self) -> &str {
        &self.config.extra_usage
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run_sink(self.path.clone(), args)
    }

    fn is_plugin(&self) -> bool {
        true
    }

    fn is_builtin(&self) -> bool {
        false
    }
}

fn run_sink(path: String, args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (call_info, input) = evaluate_once(args)?;

    let input: Vec<Value> = input.into_vec();

    let request = JsonRpc::new("sink", (call_info, input));
    let request_raw = serde_json::to_string(&request);
    if let Ok(request_raw) = request_raw {
        if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
            let _ = writeln!(tmpfile, "{}", request_raw);
            let _ = tmpfile.flush();

            let real_path = Path::new(&path);
            let ext = real_path.extension();
            let ps1_file = match ext {
                Some(ext) => ext == "ps1",
                None => false,
            };

            // TODO: This sink may not work in powershell, trying to find
            // an example of what CallInfo would look like in this temp file
            let child = if ps1_file {
                Command::new("pwsh")
                    .args([
                        "-NoLogo",
                        "-NoProfile",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-File",
                        &real_path.to_string_lossy(),
                        tmpfile
                            .path()
                            .to_str()
                            .expect("Failed getting tmpfile path"),
                    ])
                    .spawn()
            } else {
                Command::new(path).arg(&tmpfile.path()).spawn()
            };

            if let Ok(mut child) = child {
                let _ = child.wait();

                Ok(ActionStream::empty())
            } else {
                Err(ShellError::untagged_runtime_error(
                    "Could not create process for sink command",
                ))
            }
        } else {
            Err(ShellError::untagged_runtime_error(
                "Could not open file to send sink command message",
            ))
        }
    } else {
        Err(ShellError::untagged_runtime_error(
            "Could not create message to sink command",
        ))
    }
}

/// Associated information for the call of a command, including the args passed to the command and a tag that spans the name of the command being called
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallInfo {
    /// The arguments associated with this call
    pub args: EvaluatedArgs,
    /// The tag (underline-able position) of the name of the call itself
    pub name_tag: Tag,
}

/// The set of positional and named arguments, after their values have been evaluated.
///
/// * Positional arguments are those who are given as values, without any associated flag. For example, in `foo arg1 arg2`, both `arg1` and `arg2` are positional arguments.
/// * Named arguments are those associated with a flag. For example, `foo --given bar` the named argument would be name `given` and the value `bar`.
#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct EvaluatedArgs {
    pub positional: Option<Vec<Value>>,
    pub named: Option<IndexMap<String, Value>>,
}

fn evaluate_once(args: CommandArgs) -> Result<(CallInfo, InputStream), ShellError> {
    let input = args.input;
    let call_info = evaluate_command(args.call_info, args.context)?;

    Ok((call_info, input))
}

fn evaluate_command(
    args: UnevaluatedCallInfo,
    ctx: EvaluationContext,
) -> Result<CallInfo, ShellError> {
    let name_tag = args.name_tag.clone();
    let args = evaluate_args(&args.args, &ctx)?;

    Ok(CallInfo { args, name_tag })
}

fn evaluate_args(call: &hir::Call, ctx: &EvaluationContext) -> Result<EvaluatedArgs, ShellError> {
    let mut positional_args: Vec<Value> = vec![];

    if let Some(positional) = &call.positional {
        for pos in positional {
            let result = evaluate_baseline_expr(pos, ctx)?;
            positional_args.push(result);
        }
    }

    let positional = if !positional_args.is_empty() {
        Some(positional_args)
    } else {
        None
    };

    let mut named_args = IndexMap::new();

    if let Some(named) = &call.named {
        for (name, value) in named {
            match value {
                hir::NamedValue::PresentSwitch(tag) => {
                    named_args.insert(name.clone(), UntaggedValue::boolean(true).into_value(tag));
                }
                hir::NamedValue::Value(_, expr) => {
                    named_args.insert(name.clone(), evaluate_baseline_expr(expr, ctx)?);
                }
                _ => {}
            };
        }
    }

    let named = if !named_args.is_empty() {
        Some(named_args)
    } else {
        None
    };

    Ok(EvaluatedArgs::new(positional, named))
}
