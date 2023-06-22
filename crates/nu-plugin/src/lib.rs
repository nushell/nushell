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
//! A typical plugin application will define a struct that implements the [Plugin]
//! trait and then, in it's main method, pass that [Plugin] to the [serve_plugin]
//! function, which will handle all of the input and output serialization when
//! invoked by Nushell.
//!
//! ```
//! use nu_plugin::{EvaluatedCall, EncodingType, LabeledError, Plugin, serve_plugin};
//! use nu_protocol::{PluginSignature, Value};
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn signature(&self) -> Vec<PluginSignature> {
//!         todo!();
//!     }
//!     fn run(
//!         &mut self,
//!         name: &str,
//!         call: &EvaluatedCall,
//!         input: &Value
//!     ) -> Result<Value, LabeledError> {
//!         todo!();
//!     }
//! }
//!
//! fn main() {
//!    serve_plugin(&mut MyPlugin{}, EncodingType::MsgPack)
//! }
//! ```
//!
//! Nushell's source tree contains a
//! [Plugin Example](https://github.com/nushell/nushell/tree/main/crates/nu_plugin_example)
//! that demonstrates the full range of plugin capabilities.
mod plugin;
mod protocol;
mod serializers;

pub use plugin::{serve_plugin, Plugin};
pub use protocol::{EvaluatedCall, LabeledError};
pub use serializers::EncodingType;

/// Contains functionality internal to Nushell
///
/// This module contains items that are used by other components of Nushell
/// to interface with Nushell plugins. They generally will not be of use to
/// plugin authors. Plugin authors should not typically include the `nu-internal`
/// feature.
#[cfg(feature = "nu-internal")]
pub mod nu_internal {
    pub use crate::plugin::{get_signature, PluginDeclaration};
    pub use crate::protocol::PluginResponse;
}
