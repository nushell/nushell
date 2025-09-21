use nu_protocol::{
    Example, IntoSpanned, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError,
    Signature, Value,
};

use crate::{EngineInterface, EvaluatedCall, Plugin};

/// The API for a Nushell plugin command
///
/// This is the trait that Nushell plugin commands must implement. The methods defined on
/// `PluginCommand` are invoked by [`serve_plugin`](crate::serve_plugin) during plugin registration
/// and execution.
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
/// # use nu_protocol::{LabeledError, PipelineData, Signals, Signature, Type, Value};
/// struct LowercasePlugin;
/// struct Lowercase;
///
/// impl PluginCommand for Lowercase {
///     type Plugin = LowercasePlugin;
///
///     fn name(&self) -> &str {
///         "lowercase"
///     }
///
///     fn description(&self) -> &str {
///         "Convert each string in a stream to lowercase"
///     }
///
///     fn signature(&self) -> Signature {
///         Signature::build(PluginCommand::name(self))
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
///         }, &Signals::empty())?)
///     }
/// }
///
/// # impl Plugin for LowercasePlugin {
/// #     fn version(&self) -> String {
/// #         "0.0.0".into()
/// #     }
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
    /// The type of plugin this command runs on.
    ///
    /// Since [`.run()`](Self::run) takes a reference to the plugin, it is necessary to define the
    /// type of plugin that the command expects here.
    type Plugin: Plugin;

    /// The name of the command from within Nu.
    ///
    /// In case this contains spaces, it will be treated as a subcommand.
    fn name(&self) -> &str;

    /// The signature of the command.
    ///
    /// This defines the arguments and input/output types of the command.
    fn signature(&self) -> Signature;

    /// A brief description of usage for the command.
    ///
    /// This should be short enough to fit in completion menus.
    fn description(&self) -> &str;

    /// Additional documentation for usage of the command.
    ///
    /// This is optional - any arguments documented by [`.signature()`](Self::signature) will be
    /// shown in the help page automatically. However, this can be useful for explaining things that
    /// would be too brief to include in [`.description()`](Self::description) and may span multiple lines.
    fn extra_description(&self) -> &str {
        ""
    }

    /// Search terms to help users find the command.
    ///
    /// A search query matching any of these search keywords, e.g. on `help --find`, will also
    /// show this command as a result. This may be used to suggest this command as a replacement
    /// for common system commands, or based alternate names for the functionality this command
    /// provides.
    ///
    /// For example, a `fold` command might mention `reduce` in its search terms.
    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    /// Examples, in Nu, of how the command might be used.
    ///
    /// The examples are not restricted to only including this command, and may demonstrate
    /// pipelines using the command. A `result` may optionally be provided to show users what the
    /// command would return.
    ///
    /// `PluginTest::test_command_examples()` from the
    /// [`nu-plugin-test-support`](https://docs.rs/nu-plugin-test-support) crate can be used in
    /// plugin tests to automatically test that examples produce the `result`s as specified.
    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }

    /// Perform the actual behavior of the plugin command.
    ///
    /// The behavior of the plugin is defined by the implementation of this method. When Nushell
    /// invoked the plugin [`serve_plugin`](crate::serve_plugin) will call this method and print the
    /// serialized returned value or error to stdout, which Nushell will interpret.
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
/// # use nu_protocol::{LabeledError, Signature, Type, Value};
/// struct HelloPlugin;
/// struct Hello;
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
/// # impl Plugin for HelloPlugin {
/// #     fn version(&self) -> String {
/// #         "0.0.0".into()
/// #     }
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
    /// The type of plugin this command runs on.
    ///
    /// Since [`.run()`] takes a reference to the plugin, it is necessary to define the type of
    /// plugin that the command expects here.
    type Plugin: Plugin;

    /// The name of the command from within Nu.
    ///
    /// In case this contains spaces, it will be treated as a subcommand.
    fn name(&self) -> &str;

    /// The signature of the command.
    ///
    /// This defines the arguments and input/output types of the command.
    fn signature(&self) -> Signature;

    /// A brief description of usage for the command.
    ///
    /// This should be short enough to fit in completion menus.
    fn description(&self) -> &str;

    /// Additional documentation for usage of the command.
    ///
    /// This is optional - any arguments documented by [`.signature()`] will be shown in the help
    /// page automatically. However, this can be useful for explaining things that would be too
    /// brief to include in [`.description()`](Self::description) and may span multiple lines.
    fn extra_description(&self) -> &str {
        ""
    }

    /// Search terms to help users find the command.
    ///
    /// A search query matching any of these search keywords, e.g. on `help --find`, will also
    /// show this command as a result. This may be used to suggest this command as a replacement
    /// for common system commands, or based alternate names for the functionality this command
    /// provides.
    ///
    /// For example, a `fold` command might mention `reduce` in its search terms.
    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    /// Examples, in Nu, of how the command might be used.
    ///
    /// The examples are not restricted to only including this command, and may demonstrate
    /// pipelines using the command. A `result` may optionally be provided to show users what the
    /// command would return.
    ///
    /// `PluginTest::test_command_examples()` from the
    /// [`nu-plugin-test-support`](https://docs.rs/nu-plugin-test-support) crate can be used in
    /// plugin tests to automatically test that examples produce the `result`s as specified.
    fn examples(&self) -> Vec<Example<'_>> {
        vec![]
    }

    /// Perform the actual behavior of the plugin command.
    ///
    /// The behavior of the plugin is defined by the implementation of this method. When Nushell
    /// invoked the plugin [`serve_plugin`](crate::serve_plugin) will call this method and print the
    /// serialized returned value or error to stdout, which Nushell will interpret.
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

    fn examples(&self) -> Vec<Example<'_>> {
        <Self as SimplePluginCommand>::examples(self)
    }

    fn extra_description(&self) -> &str {
        <Self as SimplePluginCommand>::extra_description(self)
    }

    fn name(&self) -> &str {
        <Self as SimplePluginCommand>::name(self)
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
        let input_value = input.into_value(span)?;
        // Wrap the output in PipelineData::value
        <Self as SimplePluginCommand>::run(self, plugin, engine, call, &input_value)
            .map(|value| PipelineData::value(value, None))
    }

    fn search_terms(&self) -> Vec<&str> {
        <Self as SimplePluginCommand>::search_terms(self)
    }

    fn signature(&self) -> Signature {
        <Self as SimplePluginCommand>::signature(self)
    }

    fn description(&self) -> &str {
        <Self as SimplePluginCommand>::description(self)
    }
}

/// Build a [`PluginSignature`] from the signature-related methods on [`PluginCommand`].
///
/// This is sent to the engine on `plugin add`.
///
/// This is not a public API.
#[doc(hidden)]
pub fn create_plugin_signature(command: &(impl PluginCommand + ?Sized)) -> PluginSignature {
    PluginSignature::new(
        // Add results of trait methods to signature
        command
            .signature()
            .description(command.description())
            .extra_description(command.extra_description())
            .search_terms(
                command
                    .search_terms()
                    .into_iter()
                    .map(String::from)
                    .collect(),
            ),
        // Convert `Example`s to `PluginExample`s
        command
            .examples()
            .into_iter()
            .map(PluginExample::from)
            .collect(),
    )
}

/// Render examples to their base value so they can be sent in the response to `Signature`.
pub(crate) fn render_examples(
    plugin: &impl Plugin,
    engine: &EngineInterface,
    examples: &mut [PluginExample],
) -> Result<(), ShellError> {
    for example in examples {
        if let Some(ref mut value) = example.result {
            value.recurse_mut(&mut |value| {
                let span = value.span();
                match value {
                    Value::Custom { .. } => {
                        let value_taken = std::mem::replace(value, Value::nothing(span));
                        let Value::Custom { val, .. } = value_taken else {
                            unreachable!()
                        };
                        *value =
                            plugin.custom_value_to_base_value(engine, val.into_spanned(span))?;
                        Ok::<_, ShellError>(())
                    }
                    _ => Ok(()),
                }
            })?;
        }
    }
    Ok(())
}
