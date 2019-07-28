use crate::{CallInfo, CommandConfig, ReturnValue, ShellError, Spanned, Value};
use serde::{Deserialize, Serialize};
use std::io;

pub trait Plugin {
    fn config(&mut self) -> Result<CommandConfig, ShellError>;
    #[allow(unused)]
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
    #[allow(unused)]
    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
    #[allow(unused)]
    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
    #[allow(unused)]
    fn sink(&mut self, call_info: CallInfo, input: Vec<Spanned<Value>>) {}

    fn quit(&mut self) {}
}

pub fn serve_plugin(plugin: &mut dyn Plugin) {
    let args = std::env::args();
    if args.len() > 1 {
        let input = std::fs::read_to_string(args.skip(1).next().unwrap());
        if let Ok(input) = input {
            let command = serde_json::from_str::<NuCommand>(&input);
            match command {
                Ok(NuCommand::config) => {
                    send_response(plugin.config());
                }
                Ok(NuCommand::begin_filter { params }) => {
                    send_response(plugin.begin_filter(params));
                }
                Ok(NuCommand::filter { params }) => {
                    send_response(plugin.filter(params));
                }
                Ok(NuCommand::end_filter) => {
                    send_response(plugin.end_filter());
                }

                Ok(NuCommand::sink { params }) => {
                    plugin.sink(params.0, params.1);
                    return;
                }
                Ok(NuCommand::quit) => {
                    plugin.quit();
                    return;
                }
                e => {
                    send_response(ShellError::string(format!(
                        "Could not handle plugin message: {} {:?}",
                        input, e
                    )));
                    return;
                }
            }
        }
    } else {
        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let command = serde_json::from_str::<NuCommand>(&input);
                    match command {
                        Ok(NuCommand::config) => {
                            send_response(plugin.config());
                        }
                        Ok(NuCommand::begin_filter { params }) => {
                            send_response(plugin.begin_filter(params));
                        }
                        Ok(NuCommand::filter { params }) => {
                            send_response(plugin.filter(params));
                        }
                        Ok(NuCommand::end_filter) => {
                            send_response(plugin.end_filter());
                        }
                        Ok(NuCommand::sink { params }) => {
                            plugin.sink(params.0, params.1);
                            break;
                        }
                        Ok(NuCommand::quit) => {
                            plugin.quit();
                            break;
                        }
                        e => {
                            send_response(ShellError::string(format!(
                                "Could not handle plugin message: {} {:?}",
                                input, e
                            )));
                            break;
                        }
                    }
                }
                e => {
                    send_response(ShellError::string(format!(
                        "Could not handle plugin message: {:?}",
                        e,
                    )));
                    break;
                }
            }
        }
    }
}

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

fn send_response<T: Serialize>(result: T) {
    let response = JsonRpc::new("response", result);
    let response_raw = serde_json::to_string(&response).unwrap();
    println!("{}", response_raw);
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
pub enum NuCommand {
    config,
    begin_filter {
        params: CallInfo,
    },
    filter {
        params: Spanned<Value>,
    },
    end_filter,
    sink {
        params: (CallInfo, Vec<Spanned<Value>>),
    },
    quit,
}
