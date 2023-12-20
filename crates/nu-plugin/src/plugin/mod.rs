mod declaration;
pub use declaration::PluginDeclaration;
use nu_engine::documentation::get_flags_section;
use std::collections::HashMap;

use crate::protocol::{CallInput, LabeledError, PluginCall, PluginData, PluginResponse};
use crate::EncodingType;
use std::env;
use std::fmt::Write;
use std::io::{BufReader, ErrorKind, Read, Write as WriteTrait};
use std::path::Path;
use std::process::{Child, ChildStdout, Command as CommandSys, Stdio};

use nu_protocol::{CustomValue, PluginSignature, ShellError, Span, Value};

use super::EvaluatedCall;

pub(crate) const OUTPUT_BUFFER_SIZE: usize = 8192;

/// Encoding scheme that defines a plugin's communication protocol with Nu
pub trait PluginEncoder: Clone {
    /// The name of the encoder (e.g., `json`)
    fn name(&self) -> &str;

    /// Serialize a `PluginCall` in the `PluginEncoder`s format
    fn encode_call(
        &self,
        plugin_call: &PluginCall,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError>;

    /// Deserialize a `PluginCall` from the `PluginEncoder`s format
    fn decode_call(&self, reader: &mut impl std::io::BufRead) -> Result<PluginCall, ShellError>;

    /// Serialize a `PluginResponse` from the plugin in this `PluginEncoder`'s preferred
    /// format
    fn encode_response(
        &self,
        plugin_response: &PluginResponse,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError>;

    /// Deserialize a `PluginResponse` from the plugin from this `PluginEncoder`'s
    /// preferred format
    fn decode_response(
        &self,
        reader: &mut impl std::io::BufRead,
    ) -> Result<PluginResponse, ShellError>;
}

pub(crate) fn create_command(path: &Path, shell: Option<&Path>) -> CommandSys {
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
        // If the child process fills its stdout buffer, it may end up waiting until the parent
        // reads the stdout, and not be able to read stdin in the meantime, causing a deadlock.
        // Writing from another thread ensures that stdout is being read at the same time, avoiding the problem.
        std::thread::spawn(move || encoding_clone.encode_call(&plugin_call, &mut stdin_writer));
    }

    // Deserialize response from plugin to extract the resulting value
    if let Some(stdout_reader) = &mut child.stdout {
        let reader = stdout_reader;
        let mut buf_read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);

        encoding.decode_response(&mut buf_read)
    } else {
        Err(ShellError::GenericError {
            error: "Error with stdout reader".into(),
            msg: "no stdout reader".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

#[doc(hidden)] // Note: not for plugin authors / only used in nu-parser
pub fn get_signature(
    path: &Path,
    shell: Option<&Path>,
    current_envs: &HashMap<String, String>,
) -> Result<Vec<PluginSignature>, ShellError> {
    let mut plugin_cmd = create_command(path, shell);
    let program_name = plugin_cmd.get_program().to_os_string().into_string();

    plugin_cmd.envs(current_envs);
    let mut child = plugin_cmd.spawn().map_err(|err| {
        let error_msg = match err.kind() {
            ErrorKind::NotFound => match program_name {
                Ok(prog_name) => {
                    format!("Can't find {prog_name}, please make sure that {prog_name} is in PATH.")
                }
                _ => {
                    format!("Error spawning child process: {err}")
                }
            },
            _ => {
                format!("Error spawning child process: {err}")
            }
        };

        ShellError::PluginFailedToLoad { msg: error_msg }
    })?;

    let mut stdin_writer = child
        .stdin
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad {
            msg: "plugin missing stdin writer".into(),
        })?;
    let mut stdout_reader = child
        .stdout
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad {
            msg: "Plugin missing stdout reader".into(),
        })?;
    let encoding = get_plugin_encoding(&mut stdout_reader)?;

    // Create message to plugin to indicate that signature is required and
    // send call to plugin asking for signature
    let encoding_clone = encoding.clone();
    // If the child process fills its stdout buffer, it may end up waiting until the parent
    // reads the stdout, and not be able to read stdin in the meantime, causing a deadlock.
    // Writing from another thread ensures that stdout is being read at the same time, avoiding the problem.
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
        _ => Err(ShellError::PluginFailedToLoad {
            msg: "Plugin missing signature".into(),
        }),
    }?;

    match child.wait() {
        Ok(_) => Ok(signatures),
        Err(err) => Err(ShellError::PluginFailedToLoad {
            msg: format!("{err}"),
        }),
    }
}

/// The basic API for a Nushell plugin
///
/// This is the trait that Nushell plugins must implement. The methods defined on
/// `Plugin` are invoked by [serve_plugin] during plugin registration and execution.
///
/// # Examples
/// Basic usage:
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, Type, Value};
/// struct HelloPlugin;
///
/// impl Plugin for HelloPlugin {
///     fn signature(&self) -> Vec<PluginSignature> {
///         let sig = PluginSignature::build("hello")
///             .input_output_type(Type::Nothing, Type::String);
///
///         vec![sig]
///     }
///
///     fn run(
///         &mut self,
///         name: &str,
///         call: &EvaluatedCall,
///         input: &Value,
///     ) -> Result<Value, LabeledError> {
///         Ok(Value::string("Hello, World!".to_owned(), call.head))
///     }
/// }
/// ```
pub trait Plugin {
    /// The signature of the plugin
    ///
    /// This method returns the [PluginSignature]s that describe the capabilities
    /// of this plugin. Since a single plugin executable can support multiple invocation
    /// patterns we return a `Vec` of signatures.
    fn signature(&self) -> Vec<PluginSignature>;

    /// Perform the actual behavior of the plugin
    ///
    /// The behavior of the plugin is defined by the implementation of this method.
    /// When Nushell invoked the plugin [serve_plugin] will call this method and
    /// print the serialized returned value or error to stdout, which Nushell will
    /// interpret.
    ///
    /// The `name` is only relevant for plugins that implement multiple commands as the
    /// invoked command will be passed in via this argument. The `call` contains
    /// metadata describing how the plugin was invoked and `input` contains the structured
    /// data passed to the command implemented by this [Plugin].
    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
}

/// Function used to implement the communication protocol between
/// nushell and an external plugin.
///
/// When creating a new plugin this function is typically used as the main entry
/// point for the plugin, e.g.
///
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, Value};
/// # struct MyPlugin;
/// # impl MyPlugin { fn new() -> Self { Self }}
/// # impl Plugin for MyPlugin {
/// #     fn signature(&self) -> Vec<PluginSignature> {todo!();}
/// #     fn run(&mut self, name: &str, call: &EvaluatedCall, input: &Value)
/// #         -> Result<Value, LabeledError> {todo!();}
/// # }
/// fn main() {
///    serve_plugin(&mut MyPlugin::new(), MsgPackSerializer)
/// }
/// ```
///
/// The object that is expected to be received by nushell is the `PluginResponse` struct.
/// The `serve_plugin` function should ensure that it is encoded correctly and sent
/// to StdOut for nushell to decode and and present its result.
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
                                .map(|custom_value| {
                                    Value::custom_value(custom_value, plugin_data.span)
                                })
                                .map_err(|err| ShellError::PluginFailedToDecode {
                                    msg: err.to_string(),
                                })
                        }
                    };

                    let value = match input {
                        Ok(input) => plugin.run(&call_info.name, &call_info.call, &input),
                        Err(err) => Err(err.into()),
                    };

                    let response = match value {
                        Ok(value) => {
                            let span = value.span();
                            match value {
                                Value::CustomValue { val, .. } => match bincode::serialize(&val) {
                                    Ok(data) => {
                                        let name = val.value_string();
                                        PluginResponse::PluginData(name, PluginData { data, span })
                                    }
                                    Err(err) => PluginResponse::Error(
                                        ShellError::PluginFailedToEncode {
                                            msg: err.to_string(),
                                        }
                                        .into(),
                                    ),
                                },
                                value => PluginResponse::Value(Box::new(value)),
                            }
                        }
                        Err(err) => PluginResponse::Error(err),
                    };
                    encoder
                        .encode_response(&response, &mut std::io::stdout())
                        .expect("Error encoding response");
                }
                PluginCall::CollapseCustomValue(plugin_data) => {
                    let response = bincode::deserialize::<Box<dyn CustomValue>>(&plugin_data.data)
                        .map_err(|err| ShellError::PluginFailedToDecode {
                            msg: err.to_string(),
                        })
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
        let res = write!(help, "\nCommand: {}", signature.sig.name)
            .and_then(|_| writeln!(help, "\nUsage:\n > {}", signature.sig.usage))
            .and_then(|_| {
                if !signature.sig.extra_usage.is_empty() {
                    writeln!(help, "\nExtra usage:\n > {}", signature.sig.extra_usage)
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                let flags = get_flags_section(None, &signature.sig, |v| format!("{:#?}", v));
                write!(help, "{flags}")
            })
            .and_then(|_| writeln!(help, "\nParameters:"))
            .and_then(|_| {
                signature
                    .sig
                    .required_positional
                    .iter()
                    .try_for_each(|positional| {
                        writeln!(
                            help,
                            "  {} <{}>: {}",
                            positional.name, positional.shape, positional.desc
                        )
                    })
            })
            .and_then(|_| {
                signature
                    .sig
                    .optional_positional
                    .iter()
                    .try_for_each(|positional| {
                        writeln!(
                            help,
                            "  (optional) {} <{}>: {}",
                            positional.name, positional.shape, positional.desc
                        )
                    })
            })
            .and_then(|_| {
                if let Some(rest_positional) = &signature.sig.rest_positional {
                    writeln!(
                        help,
                        "  ...{} <{}>: {}",
                        rest_positional.name, rest_positional.shape, rest_positional.desc
                    )
                } else {
                    Ok(())
                }
            })
            .and_then(|_| writeln!(help, "======================"));

        if res.is_err() {
            println!("{res:?}")
        }
    });

    println!("{help}")
}

pub fn get_plugin_encoding(child_stdout: &mut ChildStdout) -> Result<EncodingType, ShellError> {
    let mut length_buf = [0u8; 1];
    child_stdout
        .read_exact(&mut length_buf)
        .map_err(|e| ShellError::PluginFailedToLoad {
            msg: format!("unable to get encoding from plugin: {e}"),
        })?;

    let mut buf = vec![0u8; length_buf[0] as usize];
    child_stdout
        .read_exact(&mut buf)
        .map_err(|e| ShellError::PluginFailedToLoad {
            msg: format!("unable to get encoding from plugin: {e}"),
        })?;

    EncodingType::try_from_bytes(&buf).ok_or_else(|| {
        let encoding_for_debug = String::from_utf8_lossy(&buf);
        ShellError::PluginFailedToLoad {
            msg: format!("get unsupported plugin encoding: {encoding_for_debug}"),
        }
    })
}
