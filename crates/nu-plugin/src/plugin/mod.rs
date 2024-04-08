use crate::{
    plugin::interface::ReceivedPluginCall,
    protocol::{CallInfo, CustomValueOp, PluginCustomValue, PluginInput, PluginOutput},
    EncodingType,
};

use std::{
    cmp::Ordering,
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    io::{BufReader, BufWriter},
    ops::Deref,
    path::Path,
    process::{Child, Command as CommandSys},
    sync::{
        mpsc::{self, TrySendError},
        Arc, Mutex,
    },
    thread,
};

use nu_engine::documentation::get_flags_section;
use nu_protocol::{
    ast::Operator, CustomValue, IntoSpanned, LabeledError, PipelineData, PluginSignature,
    ShellError, Spanned, Value,
};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub use self::interface::{PluginRead, PluginWrite};
use self::{
    command::render_examples,
    communication_mode::{
        ClientCommunicationIo, CommunicationMode, PreparedServerCommunication,
        ServerCommunicationIo,
    },
    gc::PluginGc,
};

mod command;
mod communication_mode;
mod context;
mod declaration;
mod gc;
mod interface;
mod persistent;
mod source;

pub use command::{create_plugin_signature, PluginCommand, SimplePluginCommand};
pub use declaration::PluginDeclaration;
pub use interface::{
    EngineInterface, EngineInterfaceManager, Interface, InterfaceManager, PluginInterface,
    PluginInterfaceManager,
};
pub use persistent::{GetPlugin, PersistentPlugin};

pub use context::{PluginExecutionCommandContext, PluginExecutionContext};
pub use source::PluginSource;

pub(crate) const OUTPUT_BUFFER_SIZE: usize = 8192;

/// Encoder for a specific message type. Usually implemented on [`PluginInput`]
/// and [`PluginOutput`].
#[doc(hidden)]
pub trait Encoder<T>: Clone + Send + Sync {
    /// Serialize a value in the [`PluginEncoder`]s format
    ///
    /// Returns [`ShellError::IOError`] if there was a problem writing, or
    /// [`ShellError::PluginFailedToEncode`] for a serialization error.
    #[doc(hidden)]
    fn encode(&self, data: &T, writer: &mut impl std::io::Write) -> Result<(), ShellError>;

    /// Deserialize a value from the [`PluginEncoder`]'s format
    ///
    /// Returns `None` if there is no more output to receive.
    ///
    /// Returns [`ShellError::IOError`] if there was a problem reading, or
    /// [`ShellError::PluginFailedToDecode`] for a deserialization error.
    #[doc(hidden)]
    fn decode(&self, reader: &mut impl std::io::BufRead) -> Result<Option<T>, ShellError>;
}

/// Encoding scheme that defines a plugin's communication protocol with Nu
pub trait PluginEncoder: Encoder<PluginInput> + Encoder<PluginOutput> {
    /// The name of the encoder (e.g., `json`)
    fn name(&self) -> &str;
}

#[cfg(unix)]
fn quote_arg(arg: impl AsRef<OsStr>) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let arg_bytes = arg.as_ref().as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(arg_bytes.len() + 10 /* just a guess */);
    out.push(b'"');
    for &byte in arg_bytes {
        if byte == b'"' {
            // Add backslash escape
            out.push(b'\\');
        }
        out.push(byte);
    }
    out.push(b'"');
    OsString::from_vec(out)
}

#[cfg(windows)]
fn quote_arg(arg: impl AsRef<OsStr>) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let arg = arg.as_ref();
    // Windows uses wide characters (ideally UTF-16) for native strings
    let mut out: Vec<u16> = Vec::with_capacity(arg.len() + 10 /* just a guess */);
    out.push(b'"' as u16);
    for &wide_ch in arg.encode_wide() {
        if wide_ch == b'"' as u16 {
            // Add backslash escape
            out.push(b'\\' as u16);
        }
        out.push(wide_ch);
    }
    out.push(b'"' as u16);
    OsString::from_wide(&out)
}

fn create_command(path: &Path, shell: Option<&Path>, mode: &CommunicationMode) -> CommandSys {
    log::trace!("Starting plugin: {path:?}, shell = {shell:?}, mode = {mode:?}");

    let mut mode_args = Some(mode.args());

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
                    let mut combined = quote_arg(path.as_os_str());
                    if let Some(args) = mode_args.take() {
                        for arg in args {
                            combined.push(OsStr::new(" "));
                            combined.push(quote_arg(arg));
                        }
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

    // Pass mode_args, unless we consumed it already
    if let Some(mode_args) = mode_args {
        process.args(mode_args);
    }

    // Setup I/O according to the communication mode
    mode.setup_command_io(&mut process);

    // The plugin should be run in a new process group to prevent Ctrl-C from stopping it
    #[cfg(unix)]
    process.process_group(0);
    #[cfg(windows)]
    process.creation_flags(windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP.0);

    // In order to make bugs with improper use of filesystem without getting the engine current
    // directory more obvious, the plugin always starts in the directory of its executable
    if let Some(dirname) = path.parent() {
        process.current_dir(dirname);
    }

    process
}

fn make_plugin_interface(
    mut child: Child,
    comm: PreparedServerCommunication,
    source: Arc<PluginSource>,
    gc: Option<PluginGc>,
) -> Result<PluginInterface, ShellError> {
    match comm.connect(&mut child)? {
        ServerCommunicationIo::Stdio(stdin, stdout) => make_plugin_interface_with_streams(
            stdout,
            stdin,
            move || {
                let _ = child.wait();
            },
            source,
            gc,
        ),
        ServerCommunicationIo::LocalSocket { read, write } => make_plugin_interface_with_streams(
            read,
            write,
            move || {
                let _ = child.wait();
            },
            source,
            gc,
        ),
    }
}

fn make_plugin_interface_with_streams(
    mut reader: impl std::io::Read + Send + 'static,
    writer: impl std::io::Write + Send + 'static,
    after_close: impl FnOnce() + Send + 'static,
    source: Arc<PluginSource>,
    gc: Option<PluginGc>,
) -> Result<PluginInterface, ShellError> {
    let encoder = get_plugin_encoding(&mut reader)?;

    let reader = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);
    let writer = BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, writer);

    let mut manager = PluginInterfaceManager::new(source.clone(), (Mutex::new(writer), encoder));
    manager.set_garbage_collector(gc);

    let interface = manager.get_interface();
    interface.hello()?;

    // Spawn the reader on a new thread. We need to be able to read messages at the same time that
    // we write, because we are expected to be able to handle multiple messages coming in from the
    // plugin at any time, including stream messages like `Drop`.
    std::thread::Builder::new()
        .name(format!(
            "plugin interface reader ({})",
            source.identity.name()
        ))
        .spawn(move || {
            if let Err(err) = manager.consume_all((reader, encoder)) {
                log::warn!("Error in PluginInterfaceManager: {err}");
            }
            // If the loop has ended, drop the manager so everyone disconnects and then run
            // after_close
            drop(manager);
            after_close();
        })
        .map_err(|err| ShellError::PluginFailedToLoad {
            msg: format!("Failed to spawn thread for plugin: {err}"),
        })?;

    Ok(interface)
}

#[doc(hidden)] // Note: not for plugin authors / only used in nu-parser
pub fn get_signature<E, K, V>(
    plugin: Arc<PersistentPlugin>,
    envs: impl FnOnce() -> Result<E, ShellError>,
) -> Result<Vec<PluginSignature>, ShellError>
where
    E: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    plugin.get(envs)?.get_signature()
}

/// The API for a Nushell plugin
///
/// A plugin defines multiple commands, which are added to the engine when the user calls
/// `register`.
///
/// The plugin must be able to be safely shared between threads, so that multiple invocations can
/// be run in parallel. If interior mutability is desired, consider synchronization primitives such
/// as [mutexes](std::sync::Mutex) and [channels](std::sync::mpsc).
///
/// # Examples
/// Basic usage:
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{LabeledError, Signature, Type, Value};
/// struct HelloPlugin;
/// struct Hello;
///
/// impl Plugin for HelloPlugin {
///     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {
///         vec![Box::new(Hello)]
///     }
/// }
///
/// impl SimplePluginCommand for Hello {
///     type Plugin = HelloPlugin;
///
///     fn name(&self) -> &str {
///         "hello"
///     }
///
///     fn usage(&self) -> &str {
///         "Every programmer's favorite greeting"
///     }
///
///     fn signature(&self) -> Signature {
///         Signature::build(PluginCommand::name(self))
///             .input_output_type(Type::Nothing, Type::String)
///     }
///
///     fn run(
///         &self,
///         plugin: &HelloPlugin,
///         engine: &EngineInterface,
///         call: &EvaluatedCall,
///         input: &Value,
///     ) -> Result<Value, LabeledError> {
///         Ok(Value::string("Hello, World!".to_owned(), call.head))
///     }
/// }
///
/// # fn main() {
/// #     serve_plugin(&HelloPlugin{}, MsgPackSerializer)
/// # }
/// ```
pub trait Plugin: Sync {
    /// The commands supported by the plugin
    ///
    /// Each [`PluginCommand`] contains both the signature of the command and the functionality it
    /// implements.
    ///
    /// This is only called once by [`serve_plugin`] at the beginning of your plugin's execution. It
    /// is not possible to change the defined commands during runtime.
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>>;

    /// Collapse a custom value to plain old data.
    ///
    /// The default implementation of this method just calls [`CustomValue::to_base_value`], but
    /// the method can be implemented differently if accessing plugin state is desirable.
    fn custom_value_to_base_value(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        custom_value
            .item
            .to_base_value(custom_value.span)
            .map_err(LabeledError::from)
    }

    /// Follow a numbered cell path on a custom value - e.g. `value.0`.
    ///
    /// The default implementation of this method just calls [`CustomValue::follow_path_int`], but
    /// the method can be implemented differently if accessing plugin state is desirable.
    fn custom_value_follow_path_int(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        index: Spanned<usize>,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        custom_value
            .item
            .follow_path_int(custom_value.span, index.item, index.span)
            .map_err(LabeledError::from)
    }

    /// Follow a named cell path on a custom value - e.g. `value.column`.
    ///
    /// The default implementation of this method just calls [`CustomValue::follow_path_string`],
    /// but the method can be implemented differently if accessing plugin state is desirable.
    fn custom_value_follow_path_string(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        column_name: Spanned<String>,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        custom_value
            .item
            .follow_path_string(custom_value.span, column_name.item, column_name.span)
            .map_err(LabeledError::from)
    }

    /// Implement comparison logic for custom values.
    ///
    /// The default implementation of this method just calls [`CustomValue::partial_cmp`], but
    /// the method can be implemented differently if accessing plugin state is desirable.
    ///
    /// Note that returning an error here is unlikely to produce desired behavior, as `partial_cmp`
    /// lacks a way to produce an error. At the moment the engine just logs the error, and the
    /// comparison returns `None`.
    fn custom_value_partial_cmp(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
        other_value: Value,
    ) -> Result<Option<Ordering>, LabeledError> {
        let _ = engine;
        Ok(custom_value.partial_cmp(&other_value))
    }

    /// Implement functionality for an operator on a custom value.
    ///
    /// The default implementation of this method just calls [`CustomValue::operation`], but
    /// the method can be implemented differently if accessing plugin state is desirable.
    fn custom_value_operation(
        &self,
        engine: &EngineInterface,
        left: Spanned<Box<dyn CustomValue>>,
        operator: Spanned<Operator>,
        right: Value,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        left.item
            .operation(left.span, operator.item, operator.span, &right)
            .map_err(LabeledError::from)
    }

    /// Handle a notification that all copies of a custom value within the engine have been dropped.
    ///
    /// This notification is only sent if [`CustomValue::notify_plugin_on_drop`] was true. Unlike
    /// the other custom value handlers, a span is not provided.
    ///
    /// Note that a new custom value is created each time it is sent to the engine - if you intend
    /// to accept a custom value and send it back, you may need to implement some kind of unique
    /// reference counting in your plugin, as you will receive multiple drop notifications even if
    /// the data within is identical.
    ///
    /// The default implementation does nothing. Any error generated here is unlikely to be visible
    /// to the user, and will only show up in the engine's log output.
    fn custom_value_dropped(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        let _ = (engine, custom_value);
        Ok(())
    }
}

/// Function used to implement the communication protocol between nushell and an external plugin.
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
/// #     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {todo!();}
/// # }
/// fn main() {
///    serve_plugin(&MyPlugin::new(), MsgPackSerializer)
/// }
/// ```
pub fn serve_plugin(plugin: &impl Plugin, encoder: impl PluginEncoder + 'static) {
    let args: Vec<OsString> = env::args_os().skip(1).collect();

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

    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        print_help(plugin, encoder);
        std::process::exit(0)
    }

    // Implement different communication modes:
    let mode = if args[0] == "--stdio" && args.len() == 1 {
        // --stdio always supported.
        CommunicationMode::Stdio
    } else if args[0] == "--local-socket" && args.len() == 2 {
        #[cfg(feature = "local-socket")]
        {
            CommunicationMode::LocalSocket((&args[1]).into())
        }
        #[cfg(not(feature = "local-socket"))]
        {
            eprintln!("{plugin_name}: local socket mode is not supported");
            std::process::exit(1);
        }
    } else {
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
    };

    let encoder_clone = encoder.clone();

    let result = match mode.connect_as_client() {
        Ok(ClientCommunicationIo::Stdio(stdin, mut stdout)) => {
            tell_nushell_encoding(&mut stdout, &encoder).expect("failed to tell nushell encoding");
            serve_plugin_io(
                plugin,
                &plugin_name,
                move || (stdin.lock(), encoder_clone),
                move || (stdout, encoder),
            )
        }
        #[cfg(feature = "local-socket")]
        Ok(ClientCommunicationIo::LocalSocket { read, mut write }) => {
            tell_nushell_encoding(&mut write, &encoder).expect("failed to tell nushell encoding");

            let read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, read);
            let write = Mutex::new(BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, write));
            serve_plugin_io(
                plugin,
                &plugin_name,
                move || (read, encoder_clone),
                move || (write, encoder),
            )
        }
        Err(err) => {
            eprintln!("{plugin_name}: failed to connect: {err}");
            std::process::exit(1);
        }
    };

    match result {
        Ok(()) => (),
        // Write unreported errors to the console
        Err(ServePluginError::UnreportedError(err)) => {
            eprintln!("Plugin `{plugin_name}` error: {err}");
            std::process::exit(1);
        }
        Err(_) => std::process::exit(1),
    }
}

fn tell_nushell_encoding(
    writer: &mut impl std::io::Write,
    encoder: &impl PluginEncoder,
) -> Result<(), std::io::Error> {
    // tell nushell encoding.
    //
    //                         1 byte
    // encoding format: |  content-length  | content    |
    let encoding = encoder.name();
    let length = encoding.len() as u8;
    let mut encoding_content: Vec<u8> = encoding.as_bytes().to_vec();
    encoding_content.insert(0, length);
    writer.write_all(&encoding_content)?;
    writer.flush()
}

/// An error from [`serve_plugin_io()`]
#[derive(Debug, Error)]
pub enum ServePluginError {
    /// An error occurred that could not be reported to the engine.
    #[error("{0}")]
    UnreportedError(#[source] ShellError),
    /// An error occurred that could be reported to the engine.
    #[error("{0}")]
    ReportedError(#[source] ShellError),
    /// A version mismatch occurred.
    #[error("{0}")]
    Incompatible(#[source] ShellError),
    /// An I/O error occurred.
    #[error("{0}")]
    IOError(#[source] ShellError),
    /// A thread spawning error occurred.
    #[error("{0}")]
    ThreadSpawnError(#[source] std::io::Error),
}

impl From<ShellError> for ServePluginError {
    fn from(error: ShellError) -> Self {
        match error {
            ShellError::IOError { .. } => ServePluginError::IOError(error),
            ShellError::PluginFailedToLoad { .. } => ServePluginError::Incompatible(error),
            _ => ServePluginError::UnreportedError(error),
        }
    }
}

/// Convert result error to ReportedError if it can be reported to the engine.
trait TryToReport {
    type T;
    fn try_to_report(self, engine: &EngineInterface) -> Result<Self::T, ServePluginError>;
}

impl<T, E> TryToReport for Result<T, E>
where
    E: Into<ServePluginError>,
{
    type T = T;
    fn try_to_report(self, engine: &EngineInterface) -> Result<T, ServePluginError> {
        self.map_err(|e| match e.into() {
            ServePluginError::UnreportedError(err) => {
                if engine.write_response(Err(err.clone())).is_ok() {
                    ServePluginError::ReportedError(err)
                } else {
                    ServePluginError::UnreportedError(err)
                }
            }
            other => other,
        })
    }
}

/// Serve a plugin on the given input & output.
///
/// Unlike [`serve_plugin`], this doesn't assume total control over the process lifecycle / stdin /
/// stdout, and can be used for more advanced use cases.
///
/// This is not a public API.
#[doc(hidden)]
pub fn serve_plugin_io<I, O>(
    plugin: &impl Plugin,
    plugin_name: &str,
    input: impl FnOnce() -> I + Send + 'static,
    output: impl FnOnce() -> O + Send + 'static,
) -> Result<(), ServePluginError>
where
    I: PluginRead<PluginInput> + 'static,
    O: PluginWrite<PluginOutput> + 'static,
{
    let (error_tx, error_rx) = mpsc::channel();

    // Build commands map, to make running a command easier
    let mut commands: HashMap<String, _> = HashMap::new();

    for command in plugin.commands() {
        if let Some(previous) = commands.insert(command.name().into(), command) {
            eprintln!(
                "Plugin `{plugin_name}` warning: command `{}` shadowed by another command with the \
                    same name. Check your commands' `name()` methods",
                previous.name()
            );
        }
    }

    let mut manager = EngineInterfaceManager::new(output());
    let call_receiver = manager
        .take_plugin_call_receiver()
        // This expect should be totally safe, as we just created the manager
        .expect("take_plugin_call_receiver returned None");

    // We need to hold on to the interface to keep the manager alive. We can drop it at the end
    let interface = manager.get_interface();

    // Send Hello message
    interface.hello()?;

    {
        // Spawn the reader thread
        let error_tx = error_tx.clone();
        std::thread::Builder::new()
            .name("engine interface reader".into())
            .spawn(move || {
                // Report the error on the channel if we get an error
                if let Err(err) = manager.consume_all(input()) {
                    let _ = error_tx.send(ServePluginError::from(err));
                }
            })
            .map_err(ServePluginError::ThreadSpawnError)?;
    }

    // Handle each Run plugin call on a thread
    thread::scope(|scope| {
        let run = |engine, call_info| {
            let CallInfo { name, call, input } = call_info;
            let result = if let Some(command) = commands.get(&name) {
                command.run(plugin, &engine, &call, input)
            } else {
                Err(
                    LabeledError::new(format!("Plugin command not found: `{name}`")).with_label(
                        format!("plugin `{plugin_name}` doesn't have this command"),
                        call.head,
                    ),
                )
            };
            let write_result = engine
                .write_response(result)
                .and_then(|writer| writer.write())
                .try_to_report(&engine);
            if let Err(err) = write_result {
                let _ = error_tx.send(err);
            }
        };

        // As an optimization: create one thread that can be reused for Run calls in sequence
        let (run_tx, run_rx) = mpsc::sync_channel(0);
        thread::Builder::new()
            .name("plugin runner (primary)".into())
            .spawn_scoped(scope, move || {
                for (engine, call) in run_rx {
                    run(engine, call);
                }
            })
            .map_err(ServePluginError::ThreadSpawnError)?;

        for plugin_call in call_receiver {
            // Check for pending errors
            if let Ok(error) = error_rx.try_recv() {
                return Err(error);
            }

            match plugin_call {
                // Sending the signature back to nushell to create the declaration definition
                ReceivedPluginCall::Signature { engine } => {
                    let sigs = commands
                        .values()
                        .map(|command| create_plugin_signature(command.deref()))
                        .map(|mut sig| {
                            render_examples(plugin, &engine, &mut sig.examples)?;
                            Ok(sig)
                        })
                        .collect::<Result<Vec<_>, ShellError>>()
                        .try_to_report(&engine)?;
                    engine.write_signature(sigs).try_to_report(&engine)?;
                }
                // Run the plugin on a background thread, handling any input or output streams
                ReceivedPluginCall::Run { engine, call } => {
                    // Try to run it on the primary thread
                    match run_tx.try_send((engine, call)) {
                        Ok(()) => (),
                        // If the primary thread isn't ready, spawn a secondary thread to do it
                        Err(TrySendError::Full((engine, call)))
                        | Err(TrySendError::Disconnected((engine, call))) => {
                            thread::Builder::new()
                                .name("plugin runner (secondary)".into())
                                .spawn_scoped(scope, move || run(engine, call))
                                .map_err(ServePluginError::ThreadSpawnError)?;
                        }
                    }
                }
                // Do an operation on a custom value
                ReceivedPluginCall::CustomValueOp {
                    engine,
                    custom_value,
                    op,
                } => {
                    custom_value_op(plugin, &engine, custom_value, op).try_to_report(&engine)?;
                }
            }
        }

        Ok::<_, ServePluginError>(())
    })?;

    // This will stop the manager
    drop(interface);

    // Receive any error left on the channel
    if let Ok(err) = error_rx.try_recv() {
        Err(err)
    } else {
        Ok(())
    }
}

fn custom_value_op(
    plugin: &impl Plugin,
    engine: &EngineInterface,
    custom_value: Spanned<PluginCustomValue>,
    op: CustomValueOp,
) -> Result<(), ShellError> {
    let local_value = custom_value
        .item
        .deserialize_to_custom_value(custom_value.span)?
        .into_spanned(custom_value.span);
    match op {
        CustomValueOp::ToBaseValue => {
            let result = plugin
                .custom_value_to_base_value(engine, local_value)
                .map(|value| PipelineData::Value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::FollowPathInt(index) => {
            let result = plugin
                .custom_value_follow_path_int(engine, local_value, index)
                .map(|value| PipelineData::Value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::FollowPathString(column_name) => {
            let result = plugin
                .custom_value_follow_path_string(engine, local_value, column_name)
                .map(|value| PipelineData::Value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::PartialCmp(mut other_value) => {
            PluginCustomValue::deserialize_custom_values_in(&mut other_value)?;
            match plugin.custom_value_partial_cmp(engine, local_value.item, other_value) {
                Ok(ordering) => engine.write_ordering(ordering),
                Err(err) => engine
                    .write_response(Err(err))
                    .and_then(|writer| writer.write()),
            }
        }
        CustomValueOp::Operation(operator, mut right) => {
            PluginCustomValue::deserialize_custom_values_in(&mut right)?;
            let result = plugin
                .custom_value_operation(engine, local_value, operator, right)
                .map(|value| PipelineData::Value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::Dropped => {
            let result = plugin
                .custom_value_dropped(engine, local_value.item)
                .map(|_| PipelineData::Empty);
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
    }
}

fn print_help(plugin: &impl Plugin, encoder: impl PluginEncoder) {
    use std::fmt::Write;

    println!("Nushell Plugin");
    println!("Encoder: {}", encoder.name());

    let mut help = String::new();

    plugin.commands().into_iter().for_each(|command| {
        let signature = command.signature();
        let res = write!(help, "\nCommand: {}", command.name())
            .and_then(|_| writeln!(help, "\nUsage:\n > {}", command.usage()))
            .and_then(|_| {
                if !command.extra_usage().is_empty() {
                    writeln!(help, "\nExtra usage:\n > {}", command.extra_usage())
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                let flags = get_flags_section(None, &signature, |v| format!("{:#?}", v));
                write!(help, "{flags}")
            })
            .and_then(|_| writeln!(help, "\nParameters:"))
            .and_then(|_| {
                signature
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
                if let Some(rest_positional) = &signature.rest_positional {
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

pub fn get_plugin_encoding(
    child_stdout: &mut impl std::io::Read,
) -> Result<EncodingType, ShellError> {
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
