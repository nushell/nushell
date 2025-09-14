use std::{cmp::Ordering, convert::Infallible, sync::Arc};

use nu_ansi_term::Style;
use nu_cmd_lang::create_default_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_plugin::{Plugin, PluginCommand};
use nu_plugin_engine::{PluginCustomValueWithSource, PluginSource, WithSource};
use nu_plugin_protocol::PluginCustomValue;
use nu_protocol::{
    CustomValue, Example, IntoSpanned as _, LabeledError, PipelineData, ShellError, Signals, Span,
    Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_shell_error,
};

use crate::{diff::diff_by_line, fake_register::fake_register};

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
        let mut engine_state = create_default_context();
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

    /// Make additional command declarations available for use by tests.
    ///
    /// This can be used to pull in commands from `nu-cmd-lang` for example, as required.
    pub fn add_decl(
        &mut self,
        decl: Box<dyn nu_protocol::engine::Command>,
    ) -> Result<&mut Self, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        working_set.add_decl(decl);
        self.engine_state.merge_delta(working_set.render())?;
        Ok(self)
    }

    /// Evaluate some Nushell source code with the plugin commands in scope with the given input to
    /// the pipeline.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::{IntoInterruptiblePipelineData, ShellError, Signals, Span, Value};
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<(), ShellError> {
    /// let result = PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .eval_with(
    ///         "my-command",
    ///         vec![Value::test_int(42)].into_pipeline_data(Span::test_data(), Signals::empty())
    ///     )?
    ///     .into_value(Span::test_data())?;
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
        let input = match input {
            input @ PipelineData::ByteStream(..) => input,
            input => input.map(
                move |mut value| {
                    let result = PluginCustomValue::serialize_custom_values_in(&mut value)
                        // Make sure to mark them with the source so they pass correctly, too.
                        .and_then(|_| {
                            PluginCustomValueWithSource::add_source_in(&mut value, &source)
                        });
                    match result {
                        Ok(()) => value,
                        Err(err) => Value::error(err, value.span()),
                    }
                },
                &Signals::empty(),
            )?,
        };

        // Eval the block with the input
        let mut stack = Stack::new().collect_value();
        let data = eval_block::<WithoutDebug>(&self.engine_state, &mut stack, &block, input)
            .map(|p| p.body)?;
        match data {
            data @ PipelineData::ByteStream(..) => Ok(data),
            data => data.map(
                |mut value| {
                    // Make sure to deserialize custom values
                    let result = PluginCustomValueWithSource::remove_source_in(&mut value)
                        .and_then(|_| PluginCustomValue::deserialize_custom_values_in(&mut value));
                    match result {
                        Ok(()) => value,
                        Err(err) => Value::error(err, value.span()),
                    }
                },
                &Signals::empty(),
            ),
        }
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
    ///     .into_value(Span::test_data())?;
    /// assert_eq!(Value::test_string("42"), result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval(&mut self, nu_source: &str) -> Result<PipelineData, ShellError> {
        self.eval_with(nu_source, PipelineData::empty())
    }

    /// Test a list of plugin examples. Prints an error for each failing example.
    ///
    /// See [`.test_command_examples()`] for easier usage of this method on a command's examples.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nu_plugin_test_support::PluginTest;
    /// # use nu_protocol::{ShellError, Example, Value};
    /// # use nu_plugin::*;
    /// # fn test(MyPlugin: impl Plugin + Send + 'static) -> Result<(), ShellError> {
    /// PluginTest::new("my_plugin", MyPlugin.into())?
    ///     .test_examples(&[
    ///         Example {
    ///             example: "my-command",
    ///             description: "Run my-command",
    ///             result: Some(Value::test_string("my-command output")),
    ///         },
    ///     ])
    /// # }
    /// ```
    pub fn test_examples(&mut self, examples: &[Example]) -> Result<(), ShellError> {
        let mut failed = false;

        for example in examples {
            let bold = Style::new().bold();
            let mut failed_header = || {
                failed = true;
                eprintln!("{} {}", bold.paint("Example:"), example.example);
                eprintln!("{} {}", bold.paint("Description:"), example.description);
            };
            if let Some(expectation) = &example.result {
                match self.eval(example.example) {
                    Ok(data) => {
                        let mut value = data.into_value(Span::test_data())?;

                        // Set all of the spans in the value to test_data() to avoid unnecessary
                        // differences when printing
                        let _: Result<(), Infallible> = value.recurse_mut(&mut |here| {
                            here.set_span(Span::test_data());
                            Ok(())
                        });

                        // Check for equality with the result
                        if !self.value_eq(expectation, &value)? {
                            // If they're not equal, print a diff of the debug format
                            let (expectation_formatted, value_formatted) =
                                match (expectation, &value) {
                                    (
                                        Value::Custom { val: ex_val, .. },
                                        Value::Custom { val: v_val, .. },
                                    ) => {
                                        // We have to serialize both custom values before handing them to the plugin
                                        let expectation_serialized =
                                            PluginCustomValue::serialize_from_custom_value(
                                                ex_val.as_ref(),
                                                expectation.span(),
                                            )?
                                            .with_source(self.source.clone());

                                        let value_serialized =
                                            PluginCustomValue::serialize_from_custom_value(
                                                v_val.as_ref(),
                                                expectation.span(),
                                            )?
                                            .with_source(self.source.clone());

                                        let persistent =
                                            self.source.persistent(None)?.get_plugin(None)?;
                                        let expectation_base = persistent
                                            .custom_value_to_base_value(
                                                expectation_serialized
                                                    .into_spanned(expectation.span()),
                                            )?;
                                        let value_base = persistent.custom_value_to_base_value(
                                            value_serialized.into_spanned(value.span()),
                                        )?;

                                        (
                                            format!("{expectation_base:#?}"),
                                            format!("{value_base:#?}"),
                                        )
                                    }
                                    _ => (format!("{expectation:#?}"), format!("{value:#?}")),
                                };

                            let diff = diff_by_line(&expectation_formatted, &value_formatted);
                            failed_header();
                            eprintln!("{} {}", bold.paint("Result:"), diff);
                        }
                    }
                    Err(err) => {
                        // Report the error
                        failed_header();
                        report_shell_error(&self.engine_state, &err);
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
        self.test_examples(&command.examples())
    }

    /// This implements custom value comparison with `plugin.custom_value_partial_cmp()` to behave
    /// as similarly as possible to comparison in the engine.
    ///
    /// NOTE: Try to keep these reflecting the same comparison as `Value::partial_cmp` does under
    /// normal circumstances. Otherwise people will be very confused.
    fn value_eq(&self, a: &Value, b: &Value) -> Result<bool, ShellError> {
        match (a, b) {
            (Value::Custom { val, .. }, _) => {
                // We have to serialize both custom values before handing them to the plugin
                let serialized =
                    PluginCustomValue::serialize_from_custom_value(val.as_ref(), a.span())?
                        .with_source(self.source.clone());
                let mut b_serialized = b.clone();
                PluginCustomValue::serialize_custom_values_in(&mut b_serialized)?;
                PluginCustomValueWithSource::add_source_in(&mut b_serialized, &self.source)?;
                // Now get the plugin reference and execute the comparison
                let persistent = self.source.persistent(None)?.get_plugin(None)?;
                let ordering = persistent.custom_value_partial_cmp(serialized, b_serialized)?;
                Ok(matches!(
                    ordering.map(Ordering::from),
                    Some(Ordering::Equal)
                ))
            }
            // All container types need to be here except Closure.
            (Value::List { vals: a_vals, .. }, Value::List { vals: b_vals, .. }) => {
                // Must be the same length, with all elements equivalent
                Ok(a_vals.len() == b_vals.len() && {
                    for (a_el, b_el) in a_vals.iter().zip(b_vals) {
                        if !self.value_eq(a_el, b_el)? {
                            return Ok(false);
                        }
                    }
                    true
                })
            }
            (Value::Record { val: a_rec, .. }, Value::Record { val: b_rec, .. }) => {
                // Must be the same length
                if a_rec.len() != b_rec.len() {
                    return Ok(false);
                }

                // reorder cols and vals to make more logically compare.
                // more general, if two record have same col and values,
                // the order of cols shouldn't affect the equal property.
                let mut a_rec = a_rec.clone().into_owned();
                let mut b_rec = b_rec.clone().into_owned();
                a_rec.sort_cols();
                b_rec.sort_cols();

                // Check columns first
                for (a, b) in a_rec.columns().zip(b_rec.columns()) {
                    if a != b {
                        return Ok(false);
                    }
                }
                // Then check the values
                for (a, b) in a_rec.values().zip(b_rec.values()) {
                    if !self.value_eq(a, b)? {
                        return Ok(false);
                    }
                }
                // All equal, and same length
                Ok(true)
            }
            // Fall back to regular eq.
            _ => Ok(a == b),
        }
    }

    /// This implements custom value comparison with `plugin.custom_value_to_base_value()` to behave
    /// as similarly as possible to comparison in the engine.
    pub fn custom_value_to_base_value(
        &self,
        val: &dyn CustomValue,
        span: Span,
    ) -> Result<Value, ShellError> {
        let serialized = PluginCustomValue::serialize_from_custom_value(val, span)?
            .with_source(self.source.clone());
        let persistent = self.source.persistent(None)?.get_plugin(None)?;
        persistent.custom_value_to_base_value(serialized.into_spanned(span))
    }
}
