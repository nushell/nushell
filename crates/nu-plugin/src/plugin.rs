use std::process::{Command, Stdio};
use std::{fmt::Display, path::Path};

use nu_protocol::{ast::Call, Signature, Value};

//use nu_protocol::{ShellError, Value};

#[derive(Debug)]
pub struct CallInfo {
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
    Signature(Box<Signature>),
    Value(Box<Value>),
}

/// The `Plugin` trait defines the API which plugins may use to "hook" into nushell.
pub trait Plugin {}

#[derive(Debug)]
pub enum PluginError {
    MissingSignature,
    UnableToSpawn(String),
    EncodingError(String),
    DecodingError(String),
}

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PluginError::MissingSignature => write!(f, "missing signature in plugin"),
            PluginError::UnableToSpawn(err) => {
                write!(f, "error in spawned child process: {}", err)
            }
            PluginError::EncodingError(err) => {
                write!(f, "error while encoding: {}", err)
            }
            PluginError::DecodingError(err) => {
                write!(f, "error while decoding: {}", err)
            }
        }
    }
}

pub fn get_signature(path: &Path) -> Result<Signature, PluginError> {
    let mut plugin_cmd = create_command(path);

    // Both stdout and stdin are piped so we can get the information from the plugin
    plugin_cmd.stdout(Stdio::piped());
    plugin_cmd.stdin(Stdio::piped());

    match plugin_cmd.spawn() {
        Err(err) => Err(PluginError::UnableToSpawn(format!("{}", err))),
        Ok(mut child) => {
            // create message to plugin to indicate signature
            // send message to plugin
            // deserialize message with signature
            match child.wait() {
                Err(err) => Err(PluginError::UnableToSpawn(format!("{}", err))),
                Ok(_) => Ok(Signature::build("testing")),
            }
        }
    }
}

fn create_command(path: &Path) -> Command {
    //TODO. The selection of shell could be modifiable from the config file.
    if cfg!(windows) {
        let mut process = Command::new("cmd");
        process.arg("/c");
        process.arg(path);

        process
    } else {
        let mut process = Command::new("sh");
        process.arg("-c").arg(path);

        process
    }
}
