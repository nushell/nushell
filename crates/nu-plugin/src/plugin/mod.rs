use std::{
    cmp::Ordering,
    collections::HashMap,
    env,
    ffi::OsString,
    ops::Deref,
    panic::AssertUnwindSafe,
    path::Path,
    sync::mpsc::{self, TrySendError},
    thread,
};

use nu_engine::documentation::{FormatterValue, HelpStyle, get_flags_section};
use nu_plugin_core::{
    ClientCommunicationIo, CommunicationMode, InterfaceManager, PluginEncoder, PluginRead,
    PluginWrite,
};
use nu_plugin_protocol::{CallInfo, CustomValueOp, PluginCustomValue, PluginInput, PluginOutput};
use nu_protocol::{
    CustomValue, IntoSpanned, LabeledError, PipelineData, PluginMetadata, ShellError, Span,
    Spanned, Value, ast::Operator, casing::Casing,
};
use thiserror::Error;

use self::{command::render_examples, interface::ReceivedPluginCall};

mod command;
mod interface;

pub use command::{PluginCommand, SimplePluginCommand, create_plugin_signature};
pub use interface::{EngineInterface, EngineInterfaceManager};

/// This should be larger than the largest commonly sent message to avoid excessive fragmentation.
///
/// The buffers coming from byte streams are typically each 8192 bytes, so double that.
#[allow(dead_code)]
pub(crate) const OUTPUT_BUFFER_SIZE: usize = 16384;

/// The API for a Nushell plugin
///
/// A plugin defines multiple commands, which are added to the engine when the user calls
/// `plugin add`.
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
///     fn version(&self) -> String {
///         env!("CARGO_PKG_VERSION").into()
///     }
///
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
///     fn description(&self) -> &str {
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
    /// The version of the plugin.
    ///
    /// The recommended implementation, which will use the version from your crate's `Cargo.toml`
    /// file:
    ///
    /// ```no_run
    /// # use nu_plugin::{Plugin, PluginCommand};
    /// # struct MyPlugin;
    /// # impl Plugin for MyPlugin {
    /// fn version(&self) -> String {
    ///     env!("CARGO_PKG_VERSION").into()
    /// }
    /// # fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> { vec![] }
    /// # }
    /// ```
    fn version(&self) -> String;

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
        optional: bool,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        custom_value
            .item
            .follow_path_int(custom_value.span, index.item, index.span, optional)
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
        optional: bool,
        casing: Casing,
    ) -> Result<Value, LabeledError> {
        let _ = engine;
        custom_value
            .item
            .follow_path_string(
                custom_value.span,
                column_name.item,
                column_name.span,
                optional,
                casing,
            )
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

    /// Implement saving logic for a custom value.
    ///
    /// The default implementation of this method just calls [`CustomValue::save`], but
    /// the method can be implemented differently if accessing plugin state is desirable.
    fn custom_value_save(
        &self,
        engine: &EngineInterface,
        value: Spanned<Box<dyn CustomValue>>,
        path: Spanned<&Path>,
        save_call_span: Span,
    ) -> Result<(), LabeledError> {
        let _ = engine;
        value
            .item
            .save(path, value.span, save_call_span)
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
/// #     fn version(&self) -> String { "0.0.0".into() }
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
            "{}: This plugin must be run from within Nushell. See `plugin add --help` for details \
            on how to use plugins.",
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
        Ok(ClientCommunicationIo::LocalSocket {
            read_in,
            mut write_out,
        }) => {
            use std::io::{BufReader, BufWriter};
            use std::sync::Mutex;

            tell_nushell_encoding(&mut write_out, &encoder)
                .expect("failed to tell nushell encoding");

            let read = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, read_in);
            let write = Mutex::new(BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, write_out));
            serve_plugin_io(
                plugin,
                &plugin_name,
                move || (read, encoder_clone),
                move || (write, encoder),
            )
        }
        Err(err) => {
            eprintln!("{plugin_name}: failed to connect: {err:?}");
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
    /// A panic occurred.
    #[error("a panic occurred in a plugin thread")]
    Panicked,
}

impl From<ShellError> for ServePluginError {
    fn from(error: ShellError) -> Self {
        match error {
            ShellError::Io(_) => ServePluginError::IOError(error),
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
            // SAFETY: It should be okay to use `AssertUnwindSafe` here, because we don't use any
            // of the references after we catch the unwind, and immediately exit.
            let unwind_result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let CallInfo { name, call, input } = call_info;
                let result = if let Some(command) = commands.get(&name) {
                    command.run(plugin, &engine, &call, input)
                } else {
                    Err(
                        LabeledError::new(format!("Plugin command not found: `{name}`"))
                            .with_label(
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
            }));
            if unwind_result.is_err() {
                // Exit after unwind if a panic occurred
                std::process::exit(1);
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
                // Send metadata back to nushell so it can be stored with the plugin signatures
                ReceivedPluginCall::Metadata { engine } => {
                    engine
                        .write_metadata(PluginMetadata::new().with_version(plugin.version()))
                        .try_to_report(&engine)?;
                }
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
                .map(|value| PipelineData::value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::FollowPathInt { index, optional } => {
            let result = plugin
                .custom_value_follow_path_int(engine, local_value, index, optional)
                .map(|value| PipelineData::value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::FollowPathString {
            column_name,
            optional,
            casing,
        } => {
            let result = plugin
                .custom_value_follow_path_string(engine, local_value, column_name, optional, casing)
                .map(|value| PipelineData::value(value, None));
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
                .map(|value| PipelineData::value(value, None));
            engine
                .write_response(result)
                .and_then(|writer| writer.write())
        }
        CustomValueOp::Save {
            path,
            save_call_span,
        } => {
            let path = Spanned {
                item: path.item.as_path(),
                span: path.span,
            };
            let result = plugin.custom_value_save(engine, local_value, path, save_call_span);
            engine.write_ok(result)
        }
        CustomValueOp::Dropped => {
            let result = plugin
                .custom_value_dropped(engine, local_value.item)
                .map(|_| PipelineData::empty());
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
    println!("Version: {}", plugin.version());

    // Determine the plugin name
    let exe = std::env::current_exe().ok();
    let plugin_name: String = exe
        .as_ref()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(unknown)".into());
    println!("Plugin file path: {plugin_name}");

    let mut help = String::new();
    let help_style = HelpStyle::default();

    plugin.commands().into_iter().for_each(|command| {
        let signature = command.signature();
        let res = write!(help, "\nCommand: {}", command.name())
            .and_then(|_| writeln!(help, "\nDescription:\n > {}", command.description()))
            .and_then(|_| {
                if !command.extra_description().is_empty() {
                    writeln!(
                        help,
                        "\nExtra description:\n > {}",
                        command.extra_description()
                    )
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                let flags = get_flags_section(&signature, &help_style, |v| match v {
                    FormatterValue::DefaultValue(value) => format!("{value:#?}"),
                    FormatterValue::CodeString(text) => text.to_string(),
                });
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
