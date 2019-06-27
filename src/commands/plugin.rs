use crate::errors::ShellError;
use crate::parser::registry::{CommandConfig, PositionalType};
use crate::parser::Spanned;
use crate::prelude::*;
use serde::{self, Deserialize, Serialize};
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{Read, Write};
use subprocess::Exec;

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
    response { params: VecDeque<ReturnValue> },
}

pub fn plugin(plugin_name: String, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let args = if let Some(ref positional) = args.args.positional {
        positional.clone()
    } else {
        vec![]
    };

    let mut path = std::path::PathBuf::from(".");
    path.push("target");
    path.push("debug");
    path.push(format!("nu_plugin_{}", plugin_name));
    let mut child = std::process::Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    {
        let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
        let mut stdout = child.stdout.as_mut().expect("Failed to open stdout");

        let mut reader = BufReader::new(stdout);

        let request = JsonRpc::new("init", args.clone());
        let request_raw = serde_json::to_string(&request).unwrap();
        stdin.write(format!("{}\n", request_raw).as_bytes());
    }
    let mut eos = VecDeque::new();
    eos.push_back(Value::Primitive(Primitive::EndOfStream));

    let stream = input
        .chain(eos)
        .map(move |v| match v {
            Value::Primitive(Primitive::EndOfStream) => {
                let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let mut stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);
                let request: JsonRpc<std::vec::Vec<Value>> = JsonRpc::new("quit", vec![]);
                let request_raw = serde_json::to_string(&request).unwrap();
                stdin.write(format!("{}\n", request_raw).as_bytes());

                VecDeque::new()
            }
            _ => {
                let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let mut stdout = child.stdout.as_mut().expect("Failed to open stdout");

                let mut reader = BufReader::new(stdout);

                let request = JsonRpc::new("filter", v);
                let request_raw = serde_json::to_string(&request).unwrap();
                stdin.write(format!("{}\n", request_raw).as_bytes());

                let mut input = String::new();
                match reader.read_line(&mut input) {
                    Ok(_) => {
                        let response = serde_json::from_str::<NuResult>(&input);
                        match response {
                            Ok(NuResult::response { params }) => params,
                            Err(_) => {
                                let mut result = VecDeque::new();
                                result.push_back(ReturnValue::Value(Value::Error(Box::new(
                                    ShellError::string("Error while processing input"),
                                ))));
                                result
                            }
                        }
                    }
                    Err(x) => {
                        let mut result = VecDeque::new();
                        result.push_back(ReturnValue::Value(Value::Error(Box::new(
                            ShellError::string("Error while processing input"),
                        ))));
                        result
                    }
                }
            }
        })
        .flatten();

    Ok(stream.boxed())
}
