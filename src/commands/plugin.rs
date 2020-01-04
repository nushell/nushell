use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use derive_new::new;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, Scope, Signature, UntaggedValue, Value};
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

    fn run(
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

    let args = args.evaluate_once_with_scope(
        registry,
        &Scope::it_value(UntaggedValue::string("$it").into_untagged_value()),
    )?;

    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut bos: VecDeque<Value> = VecDeque::new();
    bos.push_back(UntaggedValue::Primitive(Primitive::BeginningOfStream).into_untagged_value());

    let mut eos: VecDeque<Value> = VecDeque::new();
    eos.push_back(UntaggedValue::Primitive(Primitive::EndOfStream).into_untagged_value());

    let call_info = args.call_info.clone();

    trace!("filtering :: {:?}", call_info);

    let stream = bos
        .chain(args.input.values)
        .chain(eos)
        .map(move |v| match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::BeginningOfStream),
                ..
            } => {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);

                let request = JsonRpc::new("begin_filter", call_info.clone());
                let request_raw = serde_json::to_string(&request);

                match request_raw {
                    Err(_) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::labeled_error(
                            "Could not load json from plugin",
                            "could not load json from plugin",
                            &call_info.name_tag,
                        )));
                        return result;
                    }
                    Ok(request_raw) => match stdin.write(format!("{}\n", request_raw).as_bytes()) {
                        Ok(_) => {}
                        Err(err) => {
                            let mut result = VecDeque::new();
                            result.push_back(Err(ShellError::unexpected(format!("{}", err))));
                            return result;
                        }
                    },
                }

                let mut input = String::new();
                match reader.read_line(&mut input) {
                    Ok(_) => {
                        let response = serde_json::from_str::<NuResult>(&input);
                        match response {
                            Ok(NuResult::response { params }) => match params {
                                Ok(params) => params,
                                Err(e) => {
                                    let mut result = VecDeque::new();
                                    result.push_back(ReturnValue::Err(e));
                                    result
                                }
                            },
                            Err(e) => {
                                let mut result = VecDeque::new();
                                result.push_back(Err(ShellError::untagged_runtime_error(format!(
                                    "Error while processing begin_filter response: {:?} {}",
                                    e, input
                                ))));
                                result
                            }
                        }
                    }
                    Err(e) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::untagged_runtime_error(format!(
                            "Error while reading begin_filter response: {:?}",
                            e
                        ))));
                        result
                    }
                }
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::EndOfStream),
                ..
            } => {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);

                let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("end_filter", vec![]);
                let request_raw = match serde_json::to_string(&request) {
                    Ok(req) => req,
                    Err(err) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::unexpected(format!("{}", err))));
                        return result;
                    }
                };

                let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error

                let mut input = String::new();
                let result = match reader.read_line(&mut input) {
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
                                            let mut result = VecDeque::new();
                                            result.push_back(Err(ShellError::untagged_runtime_error(format!(
                                                "Error while processing begin_filter response: {:?} {}",
                                                e, input
                                            ))));
                                            return result;
                                        }
                                    }

                                    params
                                }
                                Err(e) => {
                                    let mut result = VecDeque::new();
                                    result.push_back(ReturnValue::Err(e));
                                    result
                                }
                            },
                            Err(e) => {
                                let mut result = VecDeque::new();
                                result.push_back(Err(ShellError::untagged_runtime_error(format!(
                                    "Error while processing end_filter response: {:?} {}",
                                    e, input
                                ))));
                                result
                            }
                        }
                    }
                    Err(e) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::untagged_runtime_error(format!(
                            "Error while reading end_filter: {:?}",
                            e
                        ))));
                        result
                    }
                };

                let _ = child.wait();

                result
            }
            _ => {
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
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::untagged_runtime_error(format!(
                            "Error while processing filter response: {:?}",
                            e
                        ))));
                        return result;
                    }
                }

                let mut input = String::new();
                match reader.read_line(&mut input) {
                    Ok(_) => {
                        let response = serde_json::from_str::<NuResult>(&input);
                        match response {
                            Ok(NuResult::response { params }) => match params {
                                Ok(params) => params,
                                Err(e) => {
                                    let mut result = VecDeque::new();
                                    result.push_back(ReturnValue::Err(e));
                                    result
                                }
                            },
                            Err(e) => {
                                let mut result = VecDeque::new();
                                result.push_back(Err(ShellError::untagged_runtime_error(format!(
                                    "Error while processing filter response: {:?}\n== input ==\n{}",
                                    e, input
                                ))));
                                result
                            }
                        }
                    }
                    Err(e) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::untagged_runtime_error(format!(
                            "Error while reading filter response: {:?}",
                            e
                        ))));
                        result
                    }
                }
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
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

    fn run(
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
    let args = args.evaluate_once(registry)?;
    let call_info = args.call_info.clone();

    let stream = async_stream! {
        let input: Vec<Value> = args.input.values.collect().await;

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
