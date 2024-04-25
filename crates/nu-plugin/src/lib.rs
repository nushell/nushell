#![allow(clippy::needless_doctest_main)]
//! # Nu Plugin: Plugin library for Nushell
//!
//! This crate contains the interface necessary to build Nushell plugins in Rust.
//! Additionally, it contains public, but undocumented, items used by Nushell itself
//! to interface with Nushell plugins. This documentation focuses on the interface
//! needed to write an independent plugin.
//!
//! Nushell plugins are stand-alone applications that communicate with Nushell
//! over stdin and stdout using a standardizes serialization framework to exchange
//! the typed data that Nushell commands utilize natively.
//!
//! A typical plugin application will define a struct that implements the [`Plugin`]
//! trait and then, in its main method, pass that [`Plugin`] to the [`serve_plugin()`]
//! function, which will handle all of the input and output serialization when
//! invoked by Nushell.
//!
//! ```rust,no_run
//! use nu_plugin::{EvaluatedCall, MsgPackSerializer, serve_plugin};
//! use nu_plugin::{EngineInterface, Plugin, PluginCommand, SimplePluginCommand};
//! use nu_protocol::{LabeledError, Signature, Value};
//!
//! struct MyPlugin;
//! struct MyCommand;
//!
//! impl Plugin for MyPlugin {
//!     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
//!         vec![Box::new(MyCommand)]
//!     }
//! }
//!
//! impl SimplePluginCommand for MyCommand {
//!     type Plugin = MyPlugin;
//!
//!     fn name(&self) -> &str {
//!         "my-command"
//!     }
//!
//!     fn usage(&self) -> &str {
//!         todo!();
//!     }
//!
//!     fn signature(&self) -> Signature {
//!         todo!();
//!     }
//!
//!     fn run(
//!         &self,
//!         plugin: &MyPlugin,
//!         engine: &EngineInterface,
//!         call: &EvaluatedCall,
//!         input: &Value
//!     ) -> Result<Value, LabeledError> {
//!         todo!();
//!     }
//! }
//!
//! fn main() {
//!    serve_plugin(&MyPlugin{}, MsgPackSerializer)
//! }
//! ```
//!
//! Nushell's source tree contains a
//! [Plugin Example](https://github.com/nushell/nushell/tree/main/crates/nu_plugin_example)
//! that demonstrates the full range of plugin capabilities.
mod plugin;
mod protocol;
mod sequence;
mod serializers;

pub use plugin::{
    serve_plugin, EngineInterface, Plugin, PluginCommand, PluginEncoder, PluginRead, PluginWrite,
    SimplePluginCommand,
};
pub use protocol::EvaluatedCall;
pub use serializers::{json::JsonSerializer, msgpack::MsgPackSerializer};

// Used by other nu crates.
#[doc(hidden)]
pub use plugin::{
    add_plugin_to_working_set, create_plugin_signature, get_signature, load_plugin_file,
    load_plugin_registry_item, serve_plugin_io, EngineInterfaceManager, GetPlugin, Interface,
    InterfaceManager, PersistentPlugin, PluginDeclaration, PluginExecutionCommandContext,
    PluginExecutionContext, PluginInterface, PluginInterfaceManager, PluginSource,
    ServePluginError,
};
#[doc(hidden)]
pub use protocol::{PluginCustomValue, PluginInput, PluginOutput};
#[doc(hidden)]
pub use serializers::EncodingType;
#[doc(hidden)]
pub mod util;

// Used by external benchmarks.
#[doc(hidden)]
pub use plugin::Encoder;
#[doc(hidden)]
pub use protocol::PluginCallResponse;
