use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::parser::registry;
use crate::prelude::*;
use derive_new::new;
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
    config: registry::Signature,
}

impl StaticCommand for PluginCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> registry::Signature {
        self.config.clone()
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
    let args = args.evaluate_once(registry)?;

    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        let stdout = child.stdout.as_mut().expect("Failed to open stdout");

        let mut reader = BufReader::new(stdout);

        let request = JsonRpc::new("begin_filter", args.args.call_info);
        let request_raw = serde_json::to_string(&request).unwrap();
        stdin.write(format!("{}\n", request_raw).as_bytes())?;
        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(_) => {
                let response = serde_json::from_str::<NuResult>(&input);
                match response {
                    Ok(NuResult::response { params }) => match params {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e);
                        }
                    },
                    Err(e) => {
                        return Err(ShellError::string(format!(
                            "Error while processing input: {:?} {}",
                            e, input
                        )));
                    }
                }
            }
            _ => {}
        }
    }

    let mut eos: VecDeque<Spanned<Value>> = VecDeque::new();
    eos.push_back(Value::Primitive(Primitive::EndOfStream).spanned_unknown());

    let stream = args
        .input
        .values
        .chain(eos)
        .map(move |v| match v {
            Spanned {
                item: Value::Primitive(Primitive::EndOfStream),
                ..
            } => {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let _ = BufReader::new(stdout);
                let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("quit", vec![]);
                let request_raw = serde_json::to_string(&request).unwrap();
                let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error

                VecDeque::new()
            }
            _ => {
                let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);

                let request = JsonRpc::new("filter", v);
                let request_raw = serde_json::to_string(&request).unwrap();
                let _ = stdin.write(format!("{}\n", request_raw).as_bytes()); // TODO: Handle error

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
                                result.push_back(Err(ShellError::string(format!(
                                    "Error while processing input: {:?} {}",
                                    e, input
                                ))));
                                result
                            }
                        }
                    }
                    Err(e) => {
                        let mut result = VecDeque::new();
                        result.push_back(Err(ShellError::string(format!(
                            "Error while processing input: {:?}",
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
