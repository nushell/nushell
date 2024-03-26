//! Test support for [Nushell](https://nushell.sh) plugins.
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use nu_plugin::*;
//! use nu_plugin_test_support::PluginTest;
//! use nu_protocol::{PluginSignature, PipelineData, Type, Span, Value, LabeledError};
//! use nu_protocol::IntoInterruptiblePipelineData;
//!
//! struct LowercasePlugin;
//! struct Lowercase;
//!
//! impl PluginCommand for Lowercase {
//!     type Plugin = LowercasePlugin;
//!
//!     fn signature(&self) -> PluginSignature {
//!         PluginSignature::build("lowercase")
//!             .usage("Convert each string in a stream to lowercase")
//!             .input_output_type(Type::List(Type::String.into()), Type::List(Type::String.into()))
//!     }
//!
//!     fn run(
//!         &self,
//!         plugin: &LowercasePlugin,
//!         engine: &EngineInterface,
//!         call: &EvaluatedCall,
//!         input: PipelineData,
//!     ) -> Result<PipelineData, LabeledError> {
//!         let span = call.head;
//!         Ok(input.map(move |value| {
//!             value.as_str()
//!                 .map(|string| Value::string(string.to_lowercase(), span))
//!                 // Errors in a stream should be returned as values.
//!                 .unwrap_or_else(|err| Value::error(err, span))
//!         }, None)?)
//!     }
//! }
//!
//! impl Plugin for LowercasePlugin {
//!     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {
//!         vec![Box::new(Lowercase)]
//!     }
//! }
//!
//! fn test_lowercase() -> Result<(), LabeledError> {
//!     let input = vec![Value::test_string("FooBar")].into_pipeline_data(None);
//!     let output = PluginTest::new("lowercase", LowercasePlugin.into())?
//!         .eval_with("lowercase", input)?
//!         .into_value(Span::test_data());
//!
//!     assert_eq!(
//!         Value::test_list(vec![
//!             Value::test_string("foobar")
//!         ]),
//!         output
//!     );
//!     Ok(())
//! }
//! #
//! # test_lowercase().unwrap();
//! ```

mod diff;
mod fake_persistent_plugin;
mod fake_register;
mod plugin_test;
mod spawn_fake_plugin;

pub use plugin_test::PluginTest;
