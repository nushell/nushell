mod declaration;
pub use declaration::PluginDeclaration;
use nu_engine::documentation::get_flags_section;
use std::collections::HashMap;

use crate::protocol::{CallInput, LabeledError, PluginCall, PluginData, PluginResponse};
use crate::EncodingType;
use std::env;
use std::fmt::Write;
use std::io::{BufReader, Read, Write as WriteTrait};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command as CommandSys, Stdio};

use nu_protocol::{CustomValue, ShellError, Span};
use nu_protocol::{Signature, Value};

use super::EvaluatedCall;

pub(crate) const OUTPUT_BUFFER_SIZE: usize = 8192;

pub trait PluginEncoder: Clone {
    fn name(&self) -> &str;

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

pub(crate) fn create_command(path: &Path, shell: &Option<PathBuf>) -> CommandSys {
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

pub(crate) fn call_plugin(
    child: &mut Child,
    plugin_call: PluginCall,
    encoding: &EncodingType,
    span: Span,
) -> Result<PluginResponse, ShellError> {
    if let Some(mut stdin_writer) = child.stdin.take() {
        let encoding_clone = encoding.clone();
        std::thread::spawn(move || {
            // PluginCall information
            encoding_clone.encode_call(&plugin_call, &mut stdin_writer)
        });
    }

    // Deserialize response from plugin to extract the resulting value
    if let Some(stdout_reader) = &mut child.stdout {
        let reader = stdout_reader;
        let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);

        encoding.decode_response(&mut buf_read)
    } else {
        Err(ShellError::GenericError(
            "Error with stdout reader".into(),
            "no stdout reader".into(),
            Some(span),
            None,
            Vec::new(),
        ))
    }
}

pub fn get_signature(
    path: &Path,
    shell: &Option<PathBuf>,
    current_envs: &HashMap<String, String>,
) -> Result<Vec<Signature>, ShellError> {
    let mut plugin_cmd = create_command(path, shell);

    plugin_cmd.envs(current_envs);
    let mut child = plugin_cmd.spawn().map_err(|err| {
        ShellError::PluginFailedToLoad(format!("Error spawning child process: {}", err))
    })?;

    let mut stdin_writer = child
        .stdin
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad("plugin missing stdin writer".into()))?;
    let mut stdout_reader = child
        .stdout
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad("Plugin missing stdout reader".into()))?;
    let encoding = get_plugin_encoding(&mut stdout_reader)?;

    // Create message to plugin to indicate that signature is required and
    // send call to plugin asking for signature
    let encoding_clone = encoding.clone();
    std::thread::spawn(move || {
        encoding_clone.encode_call(&PluginCall::Signature, &mut stdin_writer)
    });

    // deserialize response from plugin to extract the signature
    let reader = stdout_reader;
    let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);
    let response = encoding.decode_response(&mut buf_read)?;

    let signatures = match response {
        PluginResponse::Signature(sign) => Ok(sign),
        PluginResponse::Error(err) => Err(err.into()),
        _ => Err(ShellError::PluginFailedToLoad(
            "Plugin missing signature".into(),
        )),
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
    if env::args().any(|arg| (arg == "-h") || (arg == "--help")) {
        print_help(plugin, encoder);
        std::process::exit(0)
    }

    // tell nushell encoding.
    //
    //                         1 byte
    // encoding format: |  content-length  | content    |
    {
        let mut stdout = std::io::stdout();
        let encoding = encoder.name();
        let length = encoding.len() as u8;
        let mut encoding_content: Vec<u8> = encoding.as_bytes().to_vec();
        encoding_content.insert(0, length);
        stdout
            .write_all(&encoding_content)
            .expect("Failed to tell nushell my encoding");
        stdout
            .flush()
            .expect("Failed to tell nushell my encoding when flushing stdout");
    }

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
                    let input = match call_info.input {
                        CallInput::Value(value) => Ok(value),
                        CallInput::Data(plugin_data) => {
                            bincode::deserialize::<Box<dyn CustomValue>>(&plugin_data.data)
                                .map(|custom_value| Value::CustomValue {
                                    val: custom_value,
                                    span: plugin_data.span,
                                })
                                .map_err(|err| ShellError::PluginFailedToDecode(err.to_string()))
                        }
                    };

                    let value = match input {
                        Ok(input) => plugin.run(&call_info.name, &call_info.call, &input),
                        Err(err) => Err(err.into()),
                    };

                    let response = match value {
                        Ok(Value::CustomValue { val, span }) => match bincode::serialize(&val) {
                            Ok(data) => {
                                let name = val.value_string();
                                PluginResponse::PluginData(name, PluginData { data, span })
                            }
                            Err(err) => PluginResponse::Error(
                                ShellError::PluginFailedToEncode(err.to_string()).into(),
                            ),
                        },
                        Ok(value) => PluginResponse::Value(Box::new(value)),
                        Err(err) => PluginResponse::Error(err),
                    };
                    encoder
                        .encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
                PluginCall::CollapseCustomValue(plugin_data) => {
                    let response = bincode::deserialize::<Box<dyn CustomValue>>(&plugin_data.data)
                        .map_err(|err| ShellError::PluginFailedToDecode(err.to_string()))
                        .and_then(|val| val.to_base_value(plugin_data.span))
                        .map(Box::new)
                        .map_err(LabeledError::from)
                        .map_or_else(PluginResponse::Error, PluginResponse::Value);

                    encoder
                        .encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
            }
        }
    }
}

fn print_help(plugin: &mut impl Plugin, encoder: impl PluginEncoder) {
    println!("Nushell Plugin");
    println!("Encoder: {}", encoder.name());

    let mut help = String::new();

    plugin.signature().iter().for_each(|signature| {
        let res = write!(help, "\nCommand: {}", signature.name)
            .and_then(|_| writeln!(help, "\nUsage:\n > {}", signature.usage))
            .and_then(|_| {
                if !signature.extra_usage.is_empty() {
                    writeln!(help, "\nExtra usage:\n > {}", signature.extra_usage)
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                let flags = get_flags_section(signature);
                write!(help, "{}", flags)
            })
            .and_then(|_| writeln!(help, "\nParameters:"))
            .and_then(|_| {
                signature
                    .required_positional
                    .iter()
                    .try_for_each(|positional| {
                        writeln!(
                            help,
                            "  {} <{:?}>: {}",
                            positional.name, positional.shape, positional.desc
                        )
                    })
            })
            .and_then(|_| {
                signature
                    .optional_positional
                    .iter()
                    .try_for_each(|positional| {
                        writeln!(
                            help,
                            "  (optional) {} <{:?}>: {}",
                            positional.name, positional.shape, positional.desc
                        )
                    })
            })
            .and_then(|_| {
                if let Some(rest_positional) = &signature.rest_positional {
                    writeln!(
                        help,
                        "  ...{} <{:?}>: {}",
                        rest_positional.name, rest_positional.shape, rest_positional.desc
                    )
                } else {
                    Ok(())
                }
            })
            .and_then(|_| writeln!(help, "======================"));

        if res.is_err() {
            println!("{:?}", res)
        }
    });

    println!("{}", help)
}

pub fn get_plugin_encoding(child_stdout: &mut ChildStdout) -> Result<EncodingType, ShellError> {
    let mut length_buf = [0u8; 1];
    child_stdout.read_exact(&mut length_buf).map_err(|e| {
        ShellError::PluginFailedToLoad(format!("unable to get encoding from plugin: {e}"))
    })?;

    let mut buf = vec![0u8; length_buf[0] as usize];
    child_stdout.read_exact(&mut buf).map_err(|e| {
        ShellError::PluginFailedToLoad(format!("unable to get encoding from plugin: {e}"))
    })?;

    EncodingType::try_from_bytes(&buf).ok_or_else(|| {
        let encoding_for_debug = String::from_utf8_lossy(&buf);
        ShellError::PluginFailedToLoad(format!(
            "get unsupported plugin encoding: {encoding_for_debug}"
        ))
    })
}
