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
//!     fn version(&self) -> String {
//!         env!("CARGO_PKG_VERSION").into()
//!     }
//!
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
//!     fn description(&self) -> &str {
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

#[cfg(test)]
mod test_util;

pub use plugin::{serve_plugin, EngineInterface, Plugin, PluginCommand, SimplePluginCommand};

// Re-exports. Consider semver implications carefully.
pub use nu_plugin_core::{JsonSerializer, MsgPackSerializer, PluginEncoder};
pub use nu_plugin_protocol::EvaluatedCall;

// Required by other internal crates.
#[doc(hidden)]
pub use plugin::{create_plugin_signature, serve_plugin_io};
