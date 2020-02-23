use nu_errors::ShellError;
use nu_protocol::{outln, CallInfo, ReturnValue, Signature, Value};
use serde::{Deserialize, Serialize};
use std::io;

/// The `Plugin` trait defines the API which plugins may use to "hook" into nushell.
pub trait Plugin {
    /// The `config` method is used to configure a plguin's user interface / signature.
    ///
    /// This is where the "name" of the plugin (ex `fetch`), description, any required/optional fields, and flags
    /// can be defined. This information will displayed in nushell when running help <plugin name>
    fn config(&mut self) -> Result<Signature, ShellError>;

    /// `begin_filter` is the first method to be called if the `Signature` of the plugin is configured to be filterable.
    /// Any setup required for the plugin such as parsing arguments from `CallInfo` or initializing data structures
    /// can be done here. The `CallInfo` parameter will contain data configured in the `config` method of the Plugin trait.
    fn begin_filter(&mut self, _call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    /// `filter` is called for every `Value` that is processed by the plugin.
    /// This method requires the plugin `Signature` to be configured as filterable.
    fn filter(&mut self, _input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    /// `end_filter` is the last method to be called by the plugin after all `Value`s are processed by the plugin.
    /// This method requires the plugin `Signature` to be configured as filterable.
    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    /// `sink` consumes the `Value`s that are passed in, preventing further processing.
    /// This method requires the plugin `Signature` to be configured without filtering.
    fn sink(&mut self, _call_info: CallInfo, _input: Vec<Value>) {}

    fn quit(&mut self) {}
}

pub fn serve_plugin(plugin: &mut dyn Plugin) {
    let mut args = std::env::args();
    if args.len() > 1 {
        let input = args.nth(1);

        let input = match input {
            Some(arg) => std::fs::read_to_string(arg),
            None => {
                send_response(ShellError::untagged_runtime_error("No input given."));
                return;
            }
        };

        if let Ok(input) = input {
            let command = serde_json::from_str::<NuCommand>(&input);
            match command {
                Ok(NuCommand::config) => {
                    send_response(plugin.config());
                    return;
                }
                Ok(NuCommand::begin_filter { params }) => {
                    send_response(plugin.begin_filter(params));
                }
                Ok(NuCommand::filter { params }) => {
                    send_response(plugin.filter(params));
                }
                Ok(NuCommand::end_filter) => {
                    send_response(plugin.end_filter());
                    return;
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
                    send_response(ShellError::untagged_runtime_error(format!(
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
                            break;
                        }
                        Ok(NuCommand::begin_filter { params }) => {
                            send_response(plugin.begin_filter(params));
                        }
                        Ok(NuCommand::filter { params }) => {
                            send_response(plugin.filter(params));
                        }
                        Ok(NuCommand::end_filter) => {
                            send_response(plugin.end_filter());
                            break;
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
                            send_response(ShellError::untagged_runtime_error(format!(
                                "Could not handle plugin message: {} {:?}",
                                input, e
                            )));
                            break;
                        }
                    }
                }
                e => {
                    send_response(ShellError::untagged_runtime_error(format!(
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
    let response_raw = serde_json::to_string(&response);

    match response_raw {
        Ok(response) => outln!("{}", response),
        Err(err) => outln!("{}", err),
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
pub enum NuCommand {
    config,
    begin_filter { params: CallInfo },
    filter { params: Value },
    end_filter,
    sink { params: (CallInfo, Vec<Value>) },
    quit,
}
