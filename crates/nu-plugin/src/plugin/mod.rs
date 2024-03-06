mod declaration;
pub use declaration::PluginDeclaration;
use nu_engine::documentation::get_flags_section;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};

use crate::plugin::interface::{EngineInterfaceManager, ReceivedPluginCall};
use crate::protocol::{CallInfo, CustomValueOp, LabeledError, PluginInput, PluginOutput};
use crate::EncodingType;
use std::env;
use std::fmt::Write;
use std::io::{BufReader, Read, Write as WriteTrait};
use std::path::Path;
use std::process::{Child, ChildStdout, Command as CommandSys, Stdio};

use nu_protocol::{PipelineData, PluginSignature, ShellError, Value};

mod interface;
pub(crate) use interface::PluginInterface;

mod context;
pub(crate) use context::PluginExecutionCommandContext;

mod identity;
pub(crate) use identity::PluginIdentity;

use self::interface::{InterfaceManager, PluginInterfaceManager};

use super::EvaluatedCall;

pub(crate) const OUTPUT_BUFFER_SIZE: usize = 8192;

/// Encoder for a specific message type. Usually implemented on [`PluginInput`]
/// and [`PluginOutput`].
#[doc(hidden)]
pub trait Encoder<T>: Clone + Send + Sync {
    /// Serialize a value in the [`PluginEncoder`]s format
    ///
    /// Returns [ShellError::IOError] if there was a problem writing, or
    /// [ShellError::PluginFailedToEncode] for a serialization error.
    #[doc(hidden)]
    fn encode(&self, data: &T, writer: &mut impl std::io::Write) -> Result<(), ShellError>;

    /// Deserialize a value from the [`PluginEncoder`]'s format
    ///
    /// Returns `None` if there is no more output to receive.
    ///
    /// Returns [ShellError::IOError] if there was a problem reading, or
    /// [ShellError::PluginFailedToDecode] for a deserialization error.
    #[doc(hidden)]
    fn decode(&self, reader: &mut impl std::io::BufRead) -> Result<Option<T>, ShellError>;
}

/// Encoding scheme that defines a plugin's communication protocol with Nu
pub trait PluginEncoder: Encoder<PluginInput> + Encoder<PluginOutput> {
    /// The name of the encoder (e.g., `json`)
    fn name(&self) -> &str;
}

fn create_command(path: &Path, shell: Option<&Path>) -> CommandSys {
    log::trace!("Starting plugin: {path:?}, shell = {shell:?}");

    // There is only one mode supported at the moment, but the idea is that future
    // communication methods could be supported if desirable
    let mut input_arg = Some("--stdio");

    let mut process = match (path.extension(), shell) {
        (_, Some(shell)) => {
            let mut process = std::process::Command::new(shell);
            process.arg(path);

            process
        }
        (Some(extension), None) => {
            let (shell, command_switch) = match extension.to_str() {
                Some("cmd") | Some("bat") => (Some("cmd"), Some("/c")),
                Some("sh") => (Some("sh"), Some("-c")),
                Some("py") => (Some("python"), None),
                _ => (None, None),
            };

            match (shell, command_switch) {
                (Some(shell), Some(command_switch)) => {
                    let mut process = std::process::Command::new(shell);
                    process.arg(command_switch);
                    // If `command_switch` is set, we need to pass the path + arg as one argument
                    // e.g. sh -c "nu_plugin_inc --stdio"
                    let mut combined = path.as_os_str().to_owned();
                    if let Some(arg) = input_arg.take() {
                        combined.push(OsStr::new(" "));
                        combined.push(OsStr::new(arg));
                    }
                    process.arg(combined);

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

    // Pass input_arg, unless we consumed it already
    if let Some(input_arg) = input_arg {
        process.arg(input_arg);
    }

    // Both stdout and stdin are piped so we can receive information from the plugin
    process.stdout(Stdio::piped()).stdin(Stdio::piped());

    process
}

fn make_plugin_interface(
    mut child: Child,
    identity: Arc<PluginIdentity>,
) -> Result<PluginInterface, ShellError> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad {
            msg: "Plugin missing stdin writer".into(),
        })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| ShellError::PluginFailedToLoad {
            msg: "Plugin missing stdout writer".into(),
        })?;

    let encoder = get_plugin_encoding(&mut stdout)?;

    let reader = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, stdout);

    let mut manager = PluginInterfaceManager::new(identity, (Mutex::new(stdin), encoder));
    let interface = manager.get_interface();
    interface.hello()?;

    // Spawn the reader on a new thread. We need to be able to read messages at the same time that
    // we write, because we are expected to be able to handle multiple messages coming in from the
    // plugin at any time, including stream messages like `Drop`.
    std::thread::Builder::new()
        .name("plugin interface reader".into())
        .spawn(move || {
            if let Err(err) = manager.consume_all((reader, encoder)) {
                log::warn!("Error in PluginInterfaceManager: {err}");
            }
            // If the loop has ended, drop the manager so everyone disconnects and then wait for the
            // child to exit
            drop(manager);
            let _ = child.wait();
        })
        .map_err(|err| ShellError::PluginFailedToLoad {
            msg: format!("Failed to spawn thread for plugin: {err}"),
        })?;

    Ok(interface)
}

#[doc(hidden)] // Note: not for plugin authors / only used in nu-parser
pub fn get_signature(
    path: &Path,
    shell: Option<&Path>,
    current_envs: &HashMap<String, String>,
) -> Result<Vec<PluginSignature>, ShellError> {
    Arc::new(PluginIdentity::new(path, shell.map(|s| s.to_owned())))
        .spawn(current_envs)?
        .get_signature()
}

/// The basic API for a Nushell plugin
///
/// This is the trait that Nushell plugins must implement. The methods defined on
/// `Plugin` are invoked by [serve_plugin] during plugin registration and execution.
///
/// If large amounts of data are expected to need to be received or produced, it may be more
/// appropriate to implement [StreamingPlugin] instead.
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
///         config: &Option<Value>,
///         call: &EvaluatedCall,
///         input: &Value,
///     ) -> Result<Value, LabeledError> {
///         Ok(Value::string("Hello, World!".to_owned(), call.head))
///     }
/// }
///
/// # fn main() {
/// #     serve_plugin(&mut HelloPlugin{}, MsgPackSerializer)
/// # }
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
    ///
    /// This variant does not support streaming. Consider implementing [StreamingPlugin] instead
    /// if streaming is desired.
    fn run(
        &mut self,
        name: &str,
        config: &Option<Value>,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
}

/// The streaming API for a Nushell plugin
///
/// This is a more low-level version of the [Plugin] trait that supports operating on streams of
/// data. If you don't need to operate on streams, consider using that trait instead.
///
/// The methods defined on `StreamingPlugin` are invoked by [serve_plugin] during plugin
/// registration and execution.
///
/// # Examples
/// Basic usage:
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, PipelineData, Type, Value};
/// struct LowercasePlugin;
///
/// impl StreamingPlugin for LowercasePlugin {
///     fn signature(&self) -> Vec<PluginSignature> {
///         let sig = PluginSignature::build("lowercase")
///             .usage("Convert each string in a stream to lowercase")
///             .input_output_type(Type::List(Type::String.into()), Type::List(Type::String.into()));
///
///         vec![sig]
///     }
///
///     fn run(
///         &mut self,
///         name: &str,
///         config: &Option<Value>,
///         call: &EvaluatedCall,
///         input: PipelineData,
///     ) -> Result<PipelineData, LabeledError> {
///         let span = call.head;
///         Ok(input.map(move |value| {
///             value.as_str()
///                 .map(|string| Value::string(string.to_lowercase(), span))
///                 // Errors in a stream should be returned as values.
///                 .unwrap_or_else(|err| Value::error(err, span))
///         }, None)?)
///     }
/// }
///
/// # fn main() {
/// #     serve_plugin(&mut LowercasePlugin{}, MsgPackSerializer)
/// # }
/// ```
pub trait StreamingPlugin {
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
    ///
    /// This variant expects to receive and produce [PipelineData], which allows for stream-based
    /// handling of I/O. This is recommended if the plugin is expected to transform large lists or
    /// potentially large quantities of bytes. The API is more complex however, and [Plugin] is
    /// recommended instead if this is not a concern.
    fn run(
        &mut self,
        name: &str,
        config: &Option<Value>,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError>;
}

/// All [Plugin]s can be used as [StreamingPlugin]s, but input streams will be fully consumed
/// before the plugin runs.
impl<T: Plugin> StreamingPlugin for T {
    fn signature(&self) -> Vec<PluginSignature> {
        <Self as Plugin>::signature(self)
    }

    fn run(
        &mut self,
        name: &str,
        config: &Option<Value>,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // Unwrap the PipelineData from input, consuming the potential stream, and pass it to the
        // simpler signature in Plugin
        let span = input.span().unwrap_or(call.head);
        let input_value = input.into_value(span);
        // Wrap the output in PipelineData::Value
        <Self as Plugin>::run(self, name, config, call, &input_value)
            .map(|value| PipelineData::Value(value, None))
    }
}

/// Function used to implement the communication protocol between
/// nushell and an external plugin. Both [Plugin] and [StreamingPlugin] are supported.
///
/// When creating a new plugin this function is typically used as the main entry
/// point for the plugin, e.g.
///
/// ```rust,no_run
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, Value};
/// # struct MyPlugin;
/// # impl MyPlugin { fn new() -> Self { Self }}
/// # impl Plugin for MyPlugin {
/// #     fn signature(&self) -> Vec<PluginSignature> {todo!();}
/// #     fn run(&mut self, name: &str, config: &Option<Value>, call: &EvaluatedCall, input: &Value)
/// #         -> Result<Value, LabeledError> {todo!();}
/// # }
/// fn main() {
///    serve_plugin(&mut MyPlugin::new(), MsgPackSerializer)
/// }
/// ```
pub fn serve_plugin(plugin: &mut impl StreamingPlugin, encoder: impl PluginEncoder + 'static) {
    let mut args = env::args().skip(1);
    let number_of_args = args.len();
    let first_arg = args.next();

    if number_of_args == 0
        || first_arg
            .as_ref()
            .is_some_and(|arg| arg == "-h" || arg == "--help")
    {
        print_help(plugin, encoder);
        std::process::exit(0)
    }

    // Must pass --stdio for plugin execution. Any other arg is an error to give us options in the
    // future.
    if number_of_args > 1 || !first_arg.is_some_and(|arg| arg == "--stdio") {
        eprintln!(
            "{}: This plugin must be run from within Nushell.",
            env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "plugin".into())
        );
        eprintln!(
            "If you are running from Nushell, this plugin may be incompatible with the \
            version of nushell you are using."
        );
        std::process::exit(1)
    }

    // tell nushell encoding.
    //
    //                         1 byte
    // encoding format: |  content-length  | content    |
    let mut stdout = std::io::stdout();
    {
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

    let mut manager = EngineInterfaceManager::new((stdout, encoder.clone()));
    let call_receiver = manager
        .take_plugin_call_receiver()
        // This expect should be totally safe, as we just created the manager
        .expect("take_plugin_call_receiver returned None");

    // We need to hold on to the interface to keep the manager alive. We can drop it at the end
    let interface = manager.get_interface();

    // Determine the plugin name, for errors
    let exe = std::env::current_exe().ok();

    let plugin_name: String = exe
        .as_ref()
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned())
        .map(|stem| {
            stem.strip_prefix("nu_plugin_")
                .map(|s| s.to_owned())
                .unwrap_or(stem)
        })
        .unwrap_or_else(|| "(unknown)".into());

    // Try an operation that could result in ShellError. Exit if an I/O error is encountered.
    // Try to report the error to nushell otherwise, and failing that, panic.
    macro_rules! try_or_report {
        ($interface:expr, $expr:expr) => (match $expr {
            Ok(val) => val,
            // Just exit if there is an I/O error. Most likely this just means that nushell
            // interrupted us. If not, the error probably happened on the other side too, so we
            // don't need to also report it.
            Err(ShellError::IOError { .. }) => std::process::exit(1),
            // If there is another error, try to send it to nushell and then exit.
            Err(err) => {
                let _ = $interface.write_response(Err(err.clone())).unwrap_or_else(|_| {
                    // If we can't send it to nushell, panic with it so at least we get the output
                    panic!("Plugin `{plugin_name}`: {}", err)
                });
                std::process::exit(1)
            }
        })
    }

    // Send Hello message
    try_or_report!(interface, interface.hello());

    let plugin_name_clone = plugin_name.clone();

    // Spawn the reader thread
    std::thread::Builder::new()
        .name("engine interface reader".into())
        .spawn(move || {
            if let Err(err) = manager.consume_all((std::io::stdin().lock(), encoder)) {
                // Do our best to report the read error. Most likely there is some kind of
                // incompatibility between the plugin and nushell, so it makes more sense to try to
                // report it on stderr than to send something.

                eprintln!("Plugin `{plugin_name_clone}` read error: {err}");
                std::process::exit(1);
            }
        })
        .unwrap_or_else(|err| {
            // If we fail to spawn the reader thread, we should exit
            eprintln!("Plugin `{plugin_name}` failed to launch: {err}");
            std::process::exit(1);
        });

    for plugin_call in call_receiver {
        match plugin_call {
            // Sending the signature back to nushell to create the declaration definition
            ReceivedPluginCall::Signature { engine } => {
                try_or_report!(engine, engine.write_signature(plugin.signature()));
            }
            // Run the plugin, handling any input or output streams
            ReceivedPluginCall::Run {
                engine,
                call:
                    CallInfo {
                        name,
                        config,
                        call,
                        input,
                    },
            } => {
                let result = plugin.run(&name, &config, &call, input);
                let write_result = engine
                    .write_response(result)
                    .and_then(|writer| writer.write_background());
                try_or_report!(engine, write_result);
            }
            // Do an operation on a custom value
            ReceivedPluginCall::CustomValueOp {
                engine,
                custom_value,
                op,
            } => {
                let local_value = try_or_report!(
                    engine,
                    custom_value
                        .item
                        .deserialize_to_custom_value(custom_value.span)
                );
                match op {
                    CustomValueOp::ToBaseValue => {
                        let result = local_value
                            .to_base_value(custom_value.span)
                            .map(|value| PipelineData::Value(value, None));
                        let write_result = engine
                            .write_response(result)
                            .and_then(|writer| writer.write_background());
                        try_or_report!(engine, write_result);
                    }
                }
            }
        }
    }

    // This will stop the manager
    drop(interface);
}

fn print_help(plugin: &mut impl StreamingPlugin, encoder: impl PluginEncoder) {
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
