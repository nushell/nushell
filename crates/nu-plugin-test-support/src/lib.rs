//! Test support for [Nushell](https://nushell.sh) plugins.
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use nu_plugin::*;
//! use nu_plugin_test_support::PluginTest;
//! use nu_protocol::{
//!     Example, IntoInterruptiblePipelineData, LabeledError, PipelineData, ShellError, Signals,
//!     Signature, Span, Type, Value,
//! };
//!
//! struct LowercasePlugin;
//! struct Lowercase;
//!
//! impl PluginCommand for Lowercase {
//!     type Plugin = LowercasePlugin;
//!
//!     fn name(&self) -> &str {
//!         "lowercase"
//!     }
//!
//!     fn description(&self) -> &str {
//!         "Convert each string in a stream to lowercase"
//!     }
//!
//!     fn signature(&self) -> Signature {
//!         Signature::build(self.name()).input_output_type(
//!             Type::List(Type::String.into()),
//!             Type::List(Type::String.into()),
//!         )
//!     }
//!
//!     fn examples(&self) -> Vec<Example<'_>> {
//!         vec![Example {
//!             example: r#"[Hello wORLD] | lowercase"#,
//!             description: "Lowercase a list of strings",
//!             result: Some(Value::test_list(vec![
//!                 Value::test_string("hello"),
//!                 Value::test_string("world"),
//!             ])),
//!         }]
//!     }
//!
//!     fn run(
//!         &self,
//!         _plugin: &LowercasePlugin,
//!         _engine: &EngineInterface,
//!         call: &EvaluatedCall,
//!         input: PipelineData,
//!     ) -> Result<PipelineData, LabeledError> {
//!         let span = call.head;
//!         Ok(input.map(
//!             move |value| {
//!                 value
//!                     .as_str()
//!                     .map(|string| Value::string(string.to_lowercase(), span))
//!                     // Errors in a stream should be returned as values.
//!                     .unwrap_or_else(|err| Value::error(err, span))
//!             },
//!             &Signals::empty(),
//!         )?)
//!     }
//! }
//!
//! impl Plugin for LowercasePlugin {
//!     fn version(&self) -> String {
//!         env!("CARGO_PKG_VERSION").into()
//!     }
//!
//!     fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin=Self>>> {
//!         vec![Box::new(Lowercase)]
//!     }
//! }
//!
//! // #[test]
//! fn test_examples() -> Result<(), ShellError> {
//!     PluginTest::new("lowercase", LowercasePlugin.into())?
//!         .test_command_examples(&Lowercase)
//! }
//!
//! // #[test]
//! fn test_lowercase() -> Result<(), ShellError> {
//!     let input = vec![Value::test_string("FooBar")].into_pipeline_data(Span::test_data(), Signals::empty());
//!     let output = PluginTest::new("lowercase", LowercasePlugin.into())?
//!         .eval_with("lowercase", input)?
//!         .into_value(Span::test_data())?;
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
//! # test_examples().unwrap();
//! # test_lowercase().unwrap();
//! ```

mod diff;
mod fake_persistent_plugin;
mod fake_register;
mod plugin_test;
mod spawn_fake_plugin;

pub use plugin_test::PluginTest;
