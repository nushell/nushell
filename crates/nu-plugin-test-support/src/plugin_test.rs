use std::{convert::Infallible, sync::Arc};

use difference::Changeset;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_plugin::{Plugin, PluginCommand, PluginCustomValue, PluginSource};
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error_new, LabeledError, PipelineData, PluginExample, ShellError, Span, Value,
};

use crate::fake_register::fake_register;

/// An object through which plugins can be tested.
pub struct PluginTest {
    engine_state: EngineState,
    source: Arc<PluginSource>,
    entry_num: usize,
}

impl PluginTest {
    /// Create a new test for the given `plugin` named `name`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::ShellError;
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<PluginTest, ShellError> {
    /// PluginTest::new("my_plugin", MyPlugin.into())
    /// # }
    /// ```
    pub fn new(
        name: &str,
        plugin: Arc<impl Plugin + Send + 'static>,
    ) -> Result<PluginTest, ShellError> {
        let mut engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let reg_plugin = fake_register(&mut working_set, name, plugin)?;
        let source = Arc::new(PluginSource::new(reg_plugin));

        engine_state.merge_delta(working_set.render())?;

        Ok(PluginTest {
            engine_state,
            source,
            entry_num: 1,
        })
    }

    /// Get the [`EngineState`].
    pub fn engine_state(&self) -> &EngineState {
        &self.engine_state
    }

    /// Get a mutable reference to the [`EngineState`].
    pub fn engine_state_mut(&mut self) -> &mut EngineState {
        &mut self.engine_state
    }

    /// Evaluate some Nushell source code with the plugin commands in scope with the given input to
    /// the pipeline.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::{ShellError, Span, Value, IntoInterruptiblePipelineData};
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<(), ShellError> {
    /// let result = PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .eval_with(
    ///         "my-command",
    ///         vec![Value::test_int(42)].into_pipeline_data(None)
    ///     )?
    ///     .into_value(Span::test_data());
    /// assert_eq!(Value::test_string("42"), result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval_with(
        &mut self,
        nu_source: &str,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let fname = format!("entry #{}", self.entry_num);
        self.entry_num += 1;

        // Parse the source code
        let block = parse(&mut working_set, Some(&fname), nu_source.as_bytes(), false);

        // Check for parse errors
        let error = if !working_set.parse_errors.is_empty() {
            // ShellError doesn't have ParseError, use LabeledError to contain it.
            let mut error = LabeledError::new("Example failed to parse");
            error.inner.extend(
                working_set
                    .parse_errors
                    .iter()
                    .map(LabeledError::from_diagnostic),
            );
            Some(ShellError::LabeledError(error.into()))
        } else {
            None
        };

        // Merge into state
        self.engine_state.merge_delta(working_set.render())?;

        // Return error if set. We merge the delta even if we have errors so that printing the error
        // based on the engine state still works.
        if let Some(error) = error {
            return Err(error);
        }

        // Serialize custom values in the input
        let source = self.source.clone();
        let input = input.map(
            move |mut value| match PluginCustomValue::serialize_custom_values_in(&mut value) {
                Ok(()) => {
                    // Make sure to mark them with the source so they pass correctly, too.
                    PluginCustomValue::add_source(&mut value, &source);
                    value
                }
                Err(err) => Value::error(err, value.span()),
            },
            None,
        )?;

        // Eval the block with the input
        let mut stack = Stack::new().capture();
        eval_block::<WithoutDebug>(&self.engine_state, &mut stack, &block, input)?.map(
            |mut value| {
                // Make sure to deserialize custom values
                match PluginCustomValue::deserialize_custom_values_in(&mut value) {
                    Ok(()) => value,
                    Err(err) => Value::error(err, value.span()),
                }
            },
            None,
        )
    }

    /// Evaluate some Nushell source code with the plugin commands in scope.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::{ShellError, Span, Value, IntoInterruptiblePipelineData};
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<(), ShellError> {
    /// let result = PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .eval("42 | my-command")?
    ///     .into_value(Span::test_data());
    /// assert_eq!(Value::test_string("42"), result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval(&mut self, nu_source: &str) -> Result<PipelineData, ShellError> {
        self.eval_with(nu_source, PipelineData::Empty)
    }

    /// Test a list of plugin examples. Prints an error for each failing example.
    ///
    /// See [`.test_command_examples()`] for easier usage of this method on a command's examples.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::{ShellError, PluginExample, Value};
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<(), ShellError> {
    /// PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .test_examples(&[
    ///         PluginExample {
    ///             example: "my-command".into(),
    ///             description: "Run my-command".into(),
    ///             result: Some(Value::test_string("my-command output")),
    ///         },
    ///     ])
    /// # }
    /// ```
    pub fn test_examples(&mut self, examples: &[PluginExample]) -> Result<(), ShellError> {
        let mut failed = false;

        for example in examples {
            let mut failed_header = || {
                failed = true;
                eprintln!("\x1b[1mExample:\x1b[0m {}", example.example);
                eprintln!("\x1b[1mDescription:\x1b[0m {}", example.description);
            };
            if let Some(expectation) = &example.result {
                match self.eval(&example.example) {
                    Ok(data) => {
                        let mut value = data.into_value(Span::test_data());

                        // Set all of the spans in the value to test_data() to avoid unnecessary
                        // differences when printing
                        let _: Result<(), Infallible> = value.recurse_mut(&mut |here| {
                            here.set_span(Span::test_data());
                            Ok(())
                        });

                        // Check for equality with the result
                        if *expectation != value {
                            // If they're not equal, print a diff of the debug format
                            let expectation_formatted = format!("{:#?}", expectation);
                            let value_formatted = format!("{:#?}", value);
                            let diff =
                                Changeset::new(&expectation_formatted, &value_formatted, "\n");
                            failed_header();
                            eprintln!("\x1b[1mResult:\x1b[0m {diff}");
                        } else {
                            println!("{:?}, {:?}", expectation, value);
                        }
                    }
                    Err(err) => {
                        // Report the error
                        failed_header();
                        report_error_new(&self.engine_state, &err);
                    }
                }
            }
        }

        if !failed {
            Ok(())
        } else {
            Err(ShellError::GenericError {
                error: "Some examples failed. See the error output for details".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }

    /// Test examples from a command.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::ShellError;
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static, MyCommand: impl PluginCommand) -> Result<(), ShellError> {
    /// PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .test_command_examples(&MyCommand)
    /// # }
    /// ```
    pub fn test_command_examples(
        &mut self,
        command: &impl PluginCommand,
    ) -> Result<(), ShellError> {
        self.test_examples(&command.signature().examples)
    }
}
