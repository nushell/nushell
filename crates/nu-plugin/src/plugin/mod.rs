mod declaration;
pub use declaration::PluginDeclaration;

use crate::protocol::{LabeledError, PluginCall, PluginResponse};
use crate::EncodingType;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command as CommandSys, Stdio};

use nu_protocol::ShellError;
use nu_protocol::{Signature, Value};

use super::EvaluatedCall;

const OUTPUT_BUFFER_SIZE: usize = 8192;

pub trait PluginEncoder: Clone {
    fn encode_call(
        &self,
        plugin_call: &PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError>;

    fn decode_call(&self, reader: &mut impl std::io::BufRead) -> Result<PluginCall, ShellError>;

    fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError>;

    fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError>;
}

fn create_command(path: &Path, shell: &Option<PathBuf>) -> CommandSys {
    let mut process = match (path.extension(), shell) {
        (_, Some(shell)) => {
            let mut process = std::process::Command::new(shell);
            process.arg(path);

            process
        }
        (Some(extension), None) => {
            let (shell, separator) = match extension.to_str() {
                Some("cmd") | Some("bat") => (Some("cmd"), Some("/c")),
                Some("sh") => (Some("sh"), Some("-c")),
                Some("py") => (Some("python"), None),
                _ => (None, None),
            };

            match (shell, separator) {
                (Some(shell), Some(separator)) => {
                    let mut process = std::process::Command::new(shell);
                    process.arg(separator);
                    process.arg(path);

                    process
                }
                (Some(shell), None) => {
                    let mut process = std::process::Command::new(shell);
                    process.arg(path);

                    process
                }
                _ => std::process::Command::new(path),
            }
        }
        (None, None) => std::process::Command::new(path),
    };

    // Both stdout and stdin are piped so we can receive information from the plugin
    process.stdout(Stdio::piped()).stdin(Stdio::piped());

    process
}

pub fn get_signature(
    path: &Path,
    encoding: &EncodingType,
    shell: &Option<PathBuf>,
) -> Result<Vec<Signature>, ShellError> {
    let mut plugin_cmd = create_command(path, shell);

    let mut child = plugin_cmd.spawn().map_err(|err| {
        ShellError::PluginFailedToLoad(format!("Error spawning child process: {}", err))
    })?;

    // Create message to plugin to indicate that signature is required and
    // send call to plugin asking for signature
    if let Some(mut stdin_writer) = child.stdin.take() {
        let encoding_clone = encoding.clone();
        std::thread::spawn(move || {
            encoding_clone.encode_call(&PluginCall::Signature, &mut stdin_writer)
        });
    }

    // deserialize response from plugin to extract the signature
    let signatures = if let Some(stdout_reader) = &mut child.stdout {
        let reader = stdout_reader;
        let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);
        let response = encoding.decode_response(&mut buf_read)?;

        match response {
            PluginResponse::Signature(sign) => Ok(sign),
            PluginResponse::Error(err) => Err(err.into()),
            _ => Err(ShellError::PluginFailedToLoad(
                "Plugin missing signature".into(),
            )),
        }
    } else {
        Err(ShellError::PluginFailedToLoad(
            "Plugin missing stdout reader".into(),
        ))
    }?;

    match child.wait() {
        Ok(_) => Ok(signatures),
        Err(err) => Err(ShellError::PluginFailedToLoad(format!("{}", err))),
    }
}

// The next trait and functions are part of the plugin that is being created
// The `Plugin` trait defines the API which plugins use to "hook" into nushell.
pub trait Plugin {
    fn signature(&self) -> Vec<Signature>;
    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
}

// Function used in the plugin definition for the communication protocol between
// nushell and the external plugin.
// When creating a new plugin you have to use this function as the main
// entry point for the plugin, e.g.
//
// fn main() {
//    serve_plugin(plugin)
// }
//
// where plugin is your struct that implements the Plugin trait
//
// Note. When defining a plugin in other language but Rust, you will have to compile
// the plugin.capnp schema to create the object definitions that will be returned from
// the plugin.
// The object that is expected to be received by nushell is the PluginResponse struct.
// That should be encoded correctly and sent to StdOut for nushell to decode and
// and present its result
pub fn serve_plugin(plugin: &mut impl Plugin, encoder: impl PluginEncoder) {
    let mut stdin_buf = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, std::io::stdin());
    let plugin_call = encoder.decode_call(&mut stdin_buf);

    match plugin_call {
        Err(err) => {
            let response = PluginResponse::Error(err.into());
            encoder
                .encode_response(&response, &mut std::io::stdout())
                .expect("Error encoding response");
        }
        Ok(plugin_call) => {
            match plugin_call {
                // Sending the signature back to nushell to create the declaration definition
                PluginCall::Signature => {
                    let response = PluginResponse::Signature(plugin.signature());
                    encoder
                        .encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
                PluginCall::CallInfo(call_info) => {
                    let value = plugin.run(&call_info.name, &call_info.call, &call_info.input);

                    let response = match value {
                        Ok(value) => PluginResponse::Value(Box::new(value)),
                        Err(err) => PluginResponse::Error(err),
                    };
                    encoder
                        .encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
            }
        }
    }
}
