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
    config: registry::CommandConfig,
}

impl Command for PluginCommand {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        filter_plugin(self.path.clone(), args)
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn config(&self) -> registry::CommandConfig {
        self.config.clone()
    }
}

#[derive(new)]
pub struct PluginSink {
    name: String,
    path: String,
    config: registry::CommandConfig,
}

impl Sink for PluginSink {
    fn run(&self, args: SinkCommandArgs) -> Result<(), ShellError> {
        sink_plugin(self.path.clone(), args)
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn config(&self) -> registry::CommandConfig {
        self.config.clone()
    }
}

pub fn filter_plugin(path: String, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        let stdout = child.stdout.as_mut().expect("Failed to open stdout");

        let mut reader = BufReader::new(stdout);

        let request = JsonRpc::new("begin_filter", args.args);
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

pub fn sink_plugin(path: String, args: SinkCommandArgs) -> Result<(), ShellError> {
    //use subprocess::Exec;
    let request = JsonRpc::new("sink", (args.args, args.input));
    let request_raw = serde_json::to_string(&request).unwrap();
    let mut tmpfile = tempfile::NamedTempFile::new()?;
    let _ = writeln!(tmpfile, "{}", request_raw);
    let _ = tmpfile.flush();

    let mut child = std::process::Command::new(path)
        .arg(tmpfile.path())
        .spawn()
        .expect("Failed to spawn child process");

    let _ = child.wait();

    Ok(())
}
