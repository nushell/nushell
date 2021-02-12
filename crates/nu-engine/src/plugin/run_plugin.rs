use crate::command_args::CommandArgs;
use crate::whole_stream_command::{whole_stream_command, WholeStreamCommand};
use async_trait::async_trait;
use derive_new::new;
use futures::StreamExt;
use log::trace;
use nu_errors::ShellError;
use nu_plugin::jsonrpc::JsonRpc;
use nu_protocol::{Primitive, ReturnValue, Signature, UntaggedValue, Value};
use nu_stream::{OutputStream, ToOutputStream};
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

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_filter(self.path.clone(), (args)).await
    }
}

async fn run_filter(path: String, args: CommandArgs) -> Result<OutputStream, ShellError> {
    trace!("filter_plugin :: {}", path);

    let bos = futures::stream::iter(vec![
        UntaggedValue::Primitive(Primitive::BeginningOfStream).into_untagged_value()
    ]);
    let eos = futures::stream::iter(vec![
        UntaggedValue::Primitive(Primitive::EndOfStream).into_untagged_value()
    ]);

    let args = args.evaluate_once().await?;

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
            .args(&[
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

    let call_info = args.call_info.clone();

    trace!("filtering :: {:?}", call_info);

    Ok(bos
        .chain(args.input)
        .chain(eos)
        .map(move |item| {
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
                            return OutputStream::one(Err(ShellError::labeled_error(
                                "Could not load json from plugin",
                                "could not load json from plugin",
                                &call_info.name_tag,
                            )));
                        }
                        Ok(request_raw) => {
                            match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                                Ok(_) => {}
                                Err(err) => {
                                    return OutputStream::one(Err(ShellError::unexpected(
                                        format!("{}", err),
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
                                    Ok(params) => futures::stream::iter(params).to_output_stream(),
                                    Err(e) => futures::stream::iter(vec![ReturnValue::Err(e)])
                                        .to_output_stream(),
                                },

                                Err(e) => OutputStream::one(Err(
                                    ShellError::untagged_runtime_error(format!(
                                        "Error while processing begin_filter response: {:?} {}",
                                        e, input
                                    )),
                                )),
                            }
                        }
                        Err(e) => OutputStream::one(Err(ShellError::untagged_runtime_error(
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
                            return OutputStream::one(Err(ShellError::labeled_error(
                                "Could not load json from plugin",
                                "could not load json from plugin",
                                &call_info.name_tag,
                            )));
                        }
                        Ok(request_raw) => {
                            match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                                Ok(_) => {}
                                Err(err) => {
                                    return OutputStream::one(Err(ShellError::unexpected(
                                        format!("{}", err),
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
                                    Ok(params) => futures::stream::iter(params).to_output_stream(),
                                    Err(e) => futures::stream::iter(vec![ReturnValue::Err(e)])
                                        .to_output_stream(),
                                },
                                Err(e) => futures::stream::iter(vec![Err(
                                    ShellError::untagged_runtime_error(format!(
                                        "Error while processing end_filter response: {:?} {}",
                                        e, input
                                    )),
                                )])
                                .to_output_stream(),
                            }
                        }
                        Err(e) => {
                            futures::stream::iter(vec![Err(ShellError::untagged_runtime_error(
                                format!("Error while reading end_filter response: {:?}", e),
                            ))])
                            .to_output_stream()
                        }
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
                            return OutputStream::one(Err(ShellError::untagged_runtime_error(
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
                            return OutputStream::one(Err(ShellError::untagged_runtime_error(
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
                                    Ok(params) => futures::stream::iter(params).to_output_stream(),
                                    Err(e) => futures::stream::iter(vec![ReturnValue::Err(e)])
                                        .to_output_stream(),
                                },
                                Err(e) => OutputStream::one(Err(
                                    ShellError::untagged_runtime_error(format!(
                                    "Error while processing filter response: {:?}\n== input ==\n{}",
                                    e, input
                                )),
                                )),
                            }
                        }
                        Err(e) => OutputStream::one(Err(ShellError::untagged_runtime_error(
                            format!("Error while reading filter response: {:?}", e),
                        ))),
                    }
                }
            }
        })
        .flatten()
        .to_output_stream())
}

#[derive(new)]
pub struct PluginSink {
    name: String,
    path: String,
    config: Signature,
}

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_sink(self.path.clone(), args).await
    }
}

async fn run_sink(path: String, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let call_info = args.call_info.clone();

    let input: Vec<Value> = args.input.collect().await;

    let request = JsonRpc::new("sink", (call_info.clone(), input));
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
                    .args(&[
                        "-NoLogo",
                        "-NoProfile",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-File",
                        &real_path.to_string_lossy(),
                        &tmpfile
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

                Ok(OutputStream::empty())
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
