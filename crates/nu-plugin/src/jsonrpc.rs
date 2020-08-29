use nu_protocol::{outln, CallInfo, Value};
use serde::{Deserialize, Serialize};

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

pub fn send_response<T: Serialize>(result: T) {
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
