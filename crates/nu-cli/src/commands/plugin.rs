use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use derive_new::new;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnValue, Signature, UntaggedValue, Value};
use serde::{self, Deserialize, Serialize};
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc<T> {
    jsonrpc: String,
    pub method: String,
    pub params: T,
}

impl<T> JsonRpc<T> {
    pub fn new<U: Into<String>>(method: U, params: T) -> Self {
        JsonRpc {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
pub enum NuResult {
    response {
        params: Result<VecDeque<ReturnValue>, ShellError>,
    },
}

#[derive(new)]
pub struct PluginCommand {
    name: String,
    path: String,
    config: Signature,
}

#[async_trait]
impl WholeStreamCommand for PluginCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.config.clone()
    }

    fn usage(&self) -> &str {
        &self.config.usage
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        filter_plugin(self.path.clone(), args, registry).await
    }
}

pub async fn filter_plugin(
    path: String,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    trace!("filter_plugin :: {}", path);
    let registry = registry.clone();

    let scope = args.call_info.scope.clone();

    let bos = futures::stream::iter(vec![
        UntaggedValue::Primitive(Primitive::BeginningOfStream).into_untagged_value()
    ]);
    let eos = futures::stream::iter(vec![
        UntaggedValue::Primitive(Primitive::EndOfStream).into_untagged_value()
    ]);

    let args = args.evaluate_once_with_scope(&registry, &scope).await?;

    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sink_plugin(self.path.clone(), args, registry).await
    }
}

pub async fn sink_plugin(
    path: String,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let call_info = args.call_info.clone();

    let input: Vec<Value> = args.input.collect().await;

    let request = JsonRpc::new("sink", (call_info.clone(), input));
    let request_raw = serde_json::to_string(&request);
    if let Ok(request_raw) = request_raw {
        if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
            let _ = writeln!(tmpfile, "{}", request_raw);
            let _ = tmpfile.flush();

            let child = std::process::Command::new(path).arg(tmpfile.path()).spawn();

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
