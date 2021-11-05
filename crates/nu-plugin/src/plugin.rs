use crate::plugin_call::{self, decode_call, encode_response};
use std::io::BufReader;
use std::process::{Command as CommandSys, Stdio};
use std::{fmt::Display, path::Path};

use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, Signature, Value};
use nu_protocol::{PipelineData, ShellError};

const OUTPUT_BUFFER_SIZE: usize = 8192;

#[derive(Debug)]
pub struct CallInfo {
    pub name: String,
    pub call: Call,
    pub input: Value,
}

// Information sent to the plugin
#[derive(Debug)]
pub enum PluginCall {
    Signature,
    CallInfo(Box<CallInfo>),
}

// Information received from the plugin
#[derive(Debug)]
pub enum PluginResponse {
    Error(String),
    Signature(Vec<Signature>),
    Value(Box<Value>),
}

#[derive(Debug)]
pub enum PluginError {
    MissingSignature,
    UnableToGetStdout,
    UnableToSpawn(String),
    EncodingError(String),
    DecodingError(String),
    RunTimeError(String),
}

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PluginError::MissingSignature => write!(f, "missing signature in plugin"),
            PluginError::UnableToGetStdout => write!(f, "couldn't get stdout from child process"),
            PluginError::UnableToSpawn(err) => {
                write!(f, "error in spawned child process: {}", err)
            }
            PluginError::EncodingError(err) => {
                write!(f, "error while encoding: {}", err)
            }
            PluginError::DecodingError(err) => {
                write!(f, "error while decoding: {}", err)
            }
            PluginError::RunTimeError(err) => {
                write!(f, "runtime error: {}", err)
            }
        }
    }
}

pub fn get_signature(path: &Path) -> Result<Vec<Signature>, PluginError> {
    let mut plugin_cmd = create_command(path);

    // Both stdout and stdin are piped so we can get the information from the plugin
    plugin_cmd.stdout(Stdio::piped());
    plugin_cmd.stdin(Stdio::piped());

    match plugin_cmd.spawn() {
        Err(err) => Err(PluginError::UnableToSpawn(format!("{}", err))),
        Ok(mut child) => {
            // Create message to plugin to indicate that signature is required and
            // send call to plugin asking for signature
            if let Some(mut stdin_writer) = child.stdin.take() {
                plugin_call::encode_call(&PluginCall::Signature, &mut stdin_writer)?
            }

            // deserialize response from plugin to extract the signature
            let signature = if let Some(stdout_reader) = child.stdout.take() {
                let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, stdout_reader);
                let response = plugin_call::decode_response(&mut buf_read)?;

                match response {
                    PluginResponse::Signature(sign) => Ok(sign),
                    PluginResponse::Error(msg) => Err(PluginError::DecodingError(msg)),
                    _ => Err(PluginError::DecodingError("signature not found".into())),
                }
            } else {
                Err(PluginError::UnableToGetStdout)
            }?;

            match child.wait() {
                Err(err) => Err(PluginError::UnableToSpawn(format!("{}", err))),
                Ok(_) => Ok(signature),
            }
        }
    }
}

fn create_command(path: &Path) -> CommandSys {
    //TODO. The selection of shell could be modifiable from the config file.
    if cfg!(windows) {
        let mut process = CommandSys::new("cmd");
        process.arg("/c");
        process.arg(path);

        process
    } else {
        let mut process = CommandSys::new("sh");
        process.arg("-c").arg(path);

        process
    }
}

#[derive(Debug, Clone)]
pub struct PluginDeclaration {
    name: String,
    signature: Signature,
    filename: String,
}

impl PluginDeclaration {
    pub fn new(filename: String, signature: Signature) -> Self {
        Self {
            name: signature.name.clone(),
            signature,
            filename,
        }
    }
}

impl Command for PluginDeclaration {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.signature.clone()
    }

    fn usage(&self) -> &str {
        self.signature.usage.as_str()
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Call the command with self path
        // Decode information from plugin
        // Create PipelineData
        let source_file = Path::new(&self.filename);
        let mut plugin_cmd = create_command(source_file);

        // Both stdout and stdin are piped so we can get the information from the plugin
        plugin_cmd.stdout(Stdio::piped());
        plugin_cmd.stdin(Stdio::piped());

        match plugin_cmd.spawn() {
            Err(err) => Err(ShellError::PluginError(format!("{}", err))),
            Ok(mut child) => {
                let input = match input {
                    PipelineData::Value(value) => value,
                    PipelineData::Stream(stream) => {
                        let values = stream.collect::<Vec<Value>>();

                        Value::List {
                            vals: values,
                            span: call.head,
                        }
                    }
                };

                // PluginCall information
                let plugin_call = PluginCall::CallInfo(Box::new(CallInfo {
                    name: self.name.clone(),
                    call: call.clone(),
                    input,
                }));

                // Create message to plugin to indicate that signature is required and
                // send call to plugin asking for signature
                if let Some(mut stdin_writer) = child.stdin.take() {
                    plugin_call::encode_call(&plugin_call, &mut stdin_writer)
                        .map_err(|err| ShellError::PluginError(err.to_string()))?
                }

                // Deserialize response from plugin to extract the resulting value
                let pipeline_data = if let Some(stdout_reader) = child.stdout.take() {
                    let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, stdout_reader);
                    let response = plugin_call::decode_response(&mut buf_read)
                        .map_err(|err| ShellError::PluginError(err.to_string()))?;

                    match response {
                        PluginResponse::Value(value) => {
                            Ok(PipelineData::Value(value.as_ref().clone()))
                        }
                        PluginResponse::Error(msg) => Err(PluginError::DecodingError(msg)),
                        _ => Err(PluginError::DecodingError(
                            "result value from plugin not found".into(),
                        )),
                    }
                } else {
                    Err(PluginError::UnableToGetStdout)
                }
                .map_err(|err| ShellError::PluginError(err.to_string()))?;

                match child.wait() {
                    Err(err) => Err(ShellError::PluginError(format!("{}", err))),
                    Ok(_) => Ok(pipeline_data),
                }
            }
        }
    }

    fn is_plugin(&self) -> bool {
        true
    }
}

/// The `Plugin` trait defines the API which plugins use to "hook" into nushell.
pub trait Plugin {
    fn signature(&self) -> Vec<Signature>;
    fn run(&mut self, name: &str, call: &Call, input: &Value) -> Result<Value, PluginError>;
}

// Function used in the plugin definition for the communication protocol between
// nushell and the external plugin.
// If you want to create a new plugin you have to use this function as the main
// entry point for the plugin
pub fn serve_plugin(plugin: &mut impl Plugin) {
    let mut stdin_buf = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, std::io::stdin());
    let plugin_call = decode_call(&mut stdin_buf);

    match plugin_call {
        Err(err) => {
            let response = PluginResponse::Error(err.to_string());
            encode_response(&response, &mut std::io::stdout()).expect("Error encoding response");
        }
        Ok(plugin_call) => {
            match plugin_call {
                // Sending the signature back to nushell to create the declaration definition
                PluginCall::Signature => {
                    let response = PluginResponse::Signature(plugin.signature());
                    encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
                PluginCall::CallInfo(call_info) => {
                    let value = plugin.run(&call_info.name, &call_info.call, &call_info.input);

                    let response = match value {
                        Ok(value) => PluginResponse::Value(Box::new(value)),
                        Err(err) => PluginResponse::Error(err.to_string()),
                    };
                    encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
            }
        }
    }
}
