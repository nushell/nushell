use nu_protocol::{PipelineData, PluginSignature, Value};

use crate::{EngineInterface, EvaluatedCall, LabeledError, Plugin};

/// The API for a Nushell plugin command
///
/// This is the trait that Nushell plugin commands must implement. The methods defined on
/// `PluginCommand` are invoked by [serve_plugin] during plugin registration and execution.
///
/// The plugin command must be able to be safely shared between threads, so that multiple
/// invocations can be run in parallel. If interior mutability is desired, consider synchronization
/// primitives such as [mutexes](std::sync::Mutex) and [channels](std::sync::mpsc).
///
/// This version of the trait expects stream input and output. If you have a simple plugin that just
/// operates on plain values, consider using [`SimplePluginCommand`] instead.
///
/// # Examples
/// Basic usage:
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, PipelineData, Type, Value};
/// struct LowercasePlugin;
/// struct Lowercase;
///
/// impl PluginCommand for Lowercase {
///     type Plugin = LowercasePlugin;
///
///     fn signature(&self) -> PluginSignature {
///         PluginSignature::build("lowercase")
///             .usage("Convert each string in a stream to lowercase")
///             .input_output_type(Type::List(Type::String.into()), Type::List(Type::String.into()))
///     }
///
///     fn run(
///         &self,
///         plugin: &LowercasePlugin,
///         engine: &EngineInterface,
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
/// # impl Plugin for LowercasePlugin {
/// #     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {
/// #         vec![Box::new(Lowercase)]
/// #     }
/// # }
/// #
/// # fn main() {
/// #     serve_plugin(&LowercasePlugin{}, MsgPackSerializer)
/// # }
/// ```
pub trait PluginCommand: Sync {
    /// The type of plugin this command runs on
    ///
    /// Since [`.run()`] takes a reference to the plugin, it is necessary to define the type of
    /// plugin that the command expects here.
    type Plugin: Plugin;

    /// The signature of the plugin command
    ///
    /// These are aggregated from the [`Plugin`] and sent to the engine on `register`.
    fn signature(&self) -> PluginSignature;

    /// Perform the actual behavior of the plugin command
    ///
    /// The behavior of the plugin is defined by the implementation of this method. When Nushell
    /// invoked the plugin [serve_plugin] will call this method and print the serialized returned
    /// value or error to stdout, which Nushell will interpret.
    ///
    /// `engine` provides an interface back to the Nushell engine. See [`EngineInterface`] docs for
    /// details on what methods are available.
    ///
    /// The `call` contains metadata describing how the plugin command was invoked, including
    /// arguments, and `input` contains the structured data piped into the command.
    ///
    /// This variant expects to receive and produce [`PipelineData`], which allows for stream-based
    /// handling of I/O. This is recommended if the plugin is expected to transform large
    /// lists or potentially large quantities of bytes. The API is more complex however, and
    /// [`SimplePluginCommand`] is recommended instead if this is not a concern.
    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError>;
}

/// The API for a simple Nushell plugin command
///
/// This trait is an alternative to [`PluginCommand`], and operates on values instead of streams.
/// Note that this may make handling large lists more difficult.
///
/// The plugin command must be able to be safely shared between threads, so that multiple
/// invocations can be run in parallel. If interior mutability is desired, consider synchronization
/// primitives such as [mutexes](std::sync::Mutex) and [channels](std::sync::mpsc).
///
/// # Examples
/// Basic usage:
/// ```
/// # use nu_plugin::*;
/// # use nu_protocol::{PluginSignature, Type, Value};
/// struct HelloPlugin;
/// struct Hello;
///
/// impl SimplePluginCommand for Hello {
///     type Plugin = HelloPlugin;
///
///     fn signature(&self) -> PluginSignature {
///         PluginSignature::build("hello")
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
/// # impl Plugin for HelloPlugin {
/// #     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {
/// #         vec![Box::new(Hello)]
/// #     }
/// # }
/// #
/// # fn main() {
/// #     serve_plugin(&HelloPlugin{}, MsgPackSerializer)
/// # }
/// ```
pub trait SimplePluginCommand: Sync {
    /// The type of plugin this command runs on
    ///
    /// Since [`.run()`] takes a reference to the plugin, it is necessary to define the type of
    /// plugin that the command expects here.
    type Plugin: Plugin;

    /// The signature of the plugin command
    ///
    /// These are aggregated from the [`Plugin`] and sent to the engine on `register`.
    fn signature(&self) -> PluginSignature;

    /// Perform the actual behavior of the plugin command
    ///
    /// The behavior of the plugin is defined by the implementation of this method. When Nushell
    /// invoked the plugin [serve_plugin] will call this method and print the serialized returned
    /// value or error to stdout, which Nushell will interpret.
    ///
    /// `engine` provides an interface back to the Nushell engine. See [`EngineInterface`] docs for
    /// details on what methods are available.
    ///
    /// The `call` contains metadata describing how the plugin command was invoked, including
    /// arguments, and `input` contains the structured data piped into the command.
    ///
    /// This variant does not support streaming. Consider implementing [`PluginCommand`] directly
    /// if streaming is desired.
    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
}

/// All [`SimplePluginCommand`]s can be used as [`PluginCommand`]s, but input streams will be fully
/// consumed before the plugin command runs.
impl<T> PluginCommand for T
where
    T: SimplePluginCommand,
{
    type Plugin = <Self as SimplePluginCommand>::Plugin;

    fn signature(&self) -> PluginSignature {
        <Self as SimplePluginCommand>::signature(self)
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // Unwrap the PipelineData from input, consuming the potential stream, and pass it to the
        // simpler signature in Plugin
        let span = input.span().unwrap_or(call.head);
        let input_value = input.into_value(span);
        // Wrap the output in PipelineData::Value
        <Self as SimplePluginCommand>::run(self, plugin, engine, call, &input_value)
            .map(|value| PipelineData::Value(value, None))
    }
}
