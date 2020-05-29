use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use derive_new::new;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value};
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
        filter_plugin(self.path.clone(), args, registry)
    }
}

pub fn filter_plugin(
    path: String,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    trace!("filter_plugin :: {}", path);
    let registry = registry.clone();

    let scope = args.call_info.scope.clone();

    let stream = async_stream! {
        let mut args = args.evaluate_once_with_scope(&registry, &scope).await?;

        let mut child = std::process::Command::new(path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let call_info = args.call_info.clone();

        trace!("filtering :: {:?}", call_info);

        // Beginning of the stream
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            let stdout = child.stdout.as_mut().expect("Failed to open stdout");

            let mut reader = BufReader::new(stdout);

            let request = JsonRpc::new("begin_filter", call_info.clone());
            let request_raw = serde_json::to_string(&request);

            match request_raw {
                Err(_) => {
                    yield Err(ShellError::labeled_error(
                        "Could not load json from plugin",
                        "could not load json from plugin",
                        &call_info.name_tag,
                    ));
                }
                Ok(request_raw) => match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                    Ok(_) => {}
                    Err(err) => {
                        yield Err(ShellError::unexpected(format!("{}", err)));
                    }
                },
            }

            let mut input = String::new();
            match reader.read_line(&mut input) {
                Ok(_) => {
                    let response = serde_json::from_str::<NuResult>(&input);
                    match response {
                        Ok(NuResult::response { params }) => match params {
                            Ok(params) => for param in params { yield param },
                            Err(e) => {
                                yield ReturnValue::Err(e);
                            }
                        },
                        Err(e) => {
                            yield Err(ShellError::untagged_runtime_error(format!(
                                "Error while processing begin_filter response: {:?} {}",
                                e, input
                            )));
                        }
                    }
                }
                Err(e) => {
                    yield Err(ShellError::untagged_runtime_error(format!(
                        "Error while reading begin_filter response: {:?}",
                        e
                    )));
                }
            }
        }

        // Stream contents
        {
            while let Some(v) = args.input.next().await {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);

                let request = JsonRpc::new("filter", v);
                let request_raw = serde_json::to_string(&request);
                match request_raw {
                    Ok(request_raw) => {
                        let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error
                    }
                    Err(e) => {
                        yield Err(ShellError::untagged_runtime_error(format!(
                            "Error while processing filter response: {:?}",
                            e
                        )));
                    }
                }

                let mut input = String::new();
                match reader.read_line(&mut input) {
                    Ok(_) => {
                        let response = serde_json::from_str::<NuResult>(&input);
                        match response {
                            Ok(NuResult::response { params }) => match params {
                                Ok(params) => for param in params { yield param },
                                Err(e) => {
                                    yield ReturnValue::Err(e);
                                }
                            },
                            Err(e) => {
                                yield Err(ShellError::untagged_runtime_error(format!(
                                    "Error while processing filter response: {:?}\n== input ==\n{}",
                                    e, input
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ShellError::untagged_runtime_error(format!(
                            "Error while reading filter response: {:?}",
                            e
                        )));
                    }
                }

            }
        }

        // End of the stream
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            let stdout = child.stdout.as_mut().expect("Failed to open stdout");

            let mut reader = BufReader::new(stdout);

            let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("end_filter", vec![]);
            let request_raw = match serde_json::to_string(&request) {
                Ok(req) => req,
                Err(err) => {
                    yield Err(ShellError::unexpected(format!("{}", err)));
                    return;
                }
            };

            let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error

            let mut input = String::new();
            match reader.read_line(&mut input) {
                Ok(_) => {
                    let response = serde_json::from_str::<NuResult>(&input);
                    match response {
                        Ok(NuResult::response { params }) => match params {
                            Ok(params) => {
                                let request: JsonRpc<std::vec::Vec<Value>> =
                                    JsonRpc::new("quit", vec![]);
                                let request_raw = serde_json::to_string(&request);
                                match request_raw {
                                    Ok(request_raw) => {
                                        let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error
                                    }
                                    Err(e) => {
                                        yield Err(ShellError::untagged_runtime_error(format!(
                                            "Error while processing begin_filter response: {:?} {}",
                                            e, input
                                        )));
                                        return;
                                    }
                                }

                                //yield ReturnValue::Ok(params)
                                //yield ReturnSuccess::value(Value)
                            }
                            Err(e) => {
                                yield ReturnValue::Err(e);
                            }
                        },
                        Err(e) => {
                            yield Err(ShellError::untagged_runtime_error(format!(
                                "Error while processing end_filter response: {:?} {}",
                                e, input
                            )));
                        }
                    }
                }
                Err(e) => {
                    yield Err(ShellError::untagged_runtime_error(format!(
                        "Error while reading end_filter: {:?}",
                        e
                    )));
                }
            };

            let _ = child.wait();
        }
    };

    Ok(stream.to_output_stream())
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
        sink_plugin(self.path.clone(), args, registry)
    }
}

pub fn sink_plugin(
    path: String,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;
        let call_info = args.call_info.clone();

        let input: Vec<Value> = args.input.collect().await;

        let request = JsonRpc::new("sink", (call_info.clone(), input));
        let request_raw = serde_json::to_string(&request);
        if let Ok(request_raw) = request_raw {
            if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
                let _ = writeln!(tmpfile, "{}", request_raw);
                let _ = tmpfile.flush();

                let mut child = std::process::Command::new(path)
                    .arg(tmpfile.path())
                    .spawn();

                if let Ok(mut child) = child {
                    let _ = child.wait();

                    // Needed for async_stream to type check
                    if false {
                        yield ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value());
                    }
                } else {
                    yield Err(ShellError::untagged_runtime_error("Could not create process for sink command"));
                }
            } else {
                yield Err(ShellError::untagged_runtime_error("Could not open file to send sink command message"));
            }
        } else {
            yield Err(ShellError::untagged_runtime_error("Could not create message to sink command"));
        }
    };
    Ok(OutputStream::new(stream))
}
