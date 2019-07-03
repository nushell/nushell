use indexmap::IndexMap;
use nu::{
    serve_plugin, Args, CommandConfig, Plugin, PositionalType, Primitive, ReturnValue, ShellError,
    Spanned, Value,
};
use nu::{Primitive, ReturnSuccess, ReturnValue, ShellError, Spanned, Value};
use serde::{Deserialize, Serialize};
use std::io;

struct Inc {
    inc_by: i64,
}
impl Inc {
    fn new() -> Inc {
        Inc { inc_by: 1 }
    }
}

fn send_response<T: Serialize>(result: Vec<T>) {
    let response = JsonRpc::new("response", result);
    let response_raw = serde_json::to_string(&response).unwrap();
    println!("{}", response_raw);
}

fn send_error(error: ShellError) {
    let message: ReturnValue = Err(error);
    send_response(vec![message])
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
pub enum NuCommand {
    init { params: Vec<Spanned<Value>> },
    filter { params: Value },
    quit,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut inc_by = 1;

    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let command = serde_json::from_str::<NuCommand>(&input);

                match command {
                    Ok(NuCommand::init { params }) => {
                        for param in params {
                            match param {
                                Spanned {
                                    item: Value::Primitive(Primitive::Int(i)),
                                    ..
                                } => {
                                    inc_by = i;
                                }
                                _ => {
                                    send_error(ShellError::string("Unrecognized type in params"));
                                }
                            }
                        }
                    }
                    Ok(NuCommand::filter { params }) => match params {
                        Value::Primitive(Primitive::Int(i)) => {
                            send_response(vec![ReturnSuccess::value(Value::int(i + inc_by))]);
                        }
                        Value::Primitive(Primitive::Bytes(b)) => {
                            send_response(vec![ReturnSuccess::value(Value::bytes(
                                b + inc_by as u128,
                            ))]);
                        }
                        _ => {
                            send_error(ShellError::string("Unrecognized type in stream"));
                        }
                    },
                    Ok(NuCommand::quit) => {
                        break;
                    }
                    Err(_) => {
                        send_error(ShellError::string("Unrecognized type in stream"));
                    }
                    _ => return Err(ShellError::string("Unrecognized type in params")),
                }
            }
            Err(_) => {
                send_error(ShellError::string("Unrecognized type in stream"));
            }
            Value::Primitive(Primitive::Bytes(b)) => Ok(vec![ReturnValue::Value(Value::bytes(
                b + self.inc_by as u64,
            ))]),
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}
