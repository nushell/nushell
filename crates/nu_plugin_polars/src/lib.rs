use std::cmp::Ordering;

use cache::cache_commands;
pub use cache::{Cache, Cacheable};
use dataframe::{stub::PolarsCmd, values::CustomValueType};
use nu_plugin::{EngineInterface, Plugin, PluginCommand};

mod cache;
pub mod dataframe;
pub use dataframe::*;
use nu_protocol::{ast::Operator, CustomValue, LabeledError, Spanned, Value};

use crate::{
    eager::eager_commands, expressions::expr_commands, lazy::lazy_commands,
    series::series_commands, values::PolarsPluginCustomValue,
};

#[macro_export]
macro_rules! plugin_debug {
    ($($arg:tt)*) => {{
        if std::env::var("POLARS_PLUGIN_DEBUG")
            .ok()
            .filter(|x| x == "1" || x == "true")
            .is_some() {
            eprintln!($($arg)*);
        }
    }};
}

#[derive(Default)]
pub struct PolarsPlugin {
    pub(crate) cache: Cache,
    /// For testing purposes only
    pub(crate) disable_cache_drop: bool,
}

impl Plugin for PolarsPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        let mut commands: Vec<Box<dyn PluginCommand<Plugin = Self>>> = vec![Box::new(PolarsCmd)];
        commands.append(&mut eager_commands());
        commands.append(&mut lazy_commands());
        commands.append(&mut expr_commands());
        commands.append(&mut series_commands());
        commands.append(&mut cache_commands());
        commands
    }

    fn custom_value_dropped(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        if !self.disable_cache_drop {
            let id = CustomValueType::try_from_custom_value(custom_value)?.id();
            let _ = self.cache.remove(Some(engine), &id, false);
        }
        Ok(())
    }

    fn custom_value_to_base_value(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
    ) -> Result<Value, LabeledError> {
        let result = match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => cv.custom_value_to_base_value(self, engine),
            CustomValueType::NuLazyFrame(cv) => cv.custom_value_to_base_value(self, engine),
            CustomValueType::NuExpression(cv) => cv.custom_value_to_base_value(self, engine),
            CustomValueType::NuLazyGroupBy(cv) => cv.custom_value_to_base_value(self, engine),
            CustomValueType::NuWhen(cv) => cv.custom_value_to_base_value(self, engine),
        };
        Ok(result?)
    }

    fn custom_value_operation(
        &self,
        engine: &EngineInterface,
        left: Spanned<Box<dyn CustomValue>>,
        operator: Spanned<Operator>,
        right: Value,
    ) -> Result<Value, LabeledError> {
        let result = match CustomValueType::try_from_custom_value(left.item)? {
            CustomValueType::NuDataFrame(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
            CustomValueType::NuLazyFrame(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
            CustomValueType::NuExpression(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
            CustomValueType::NuLazyGroupBy(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
            CustomValueType::NuWhen(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
        };
        Ok(result?)
    }

    fn custom_value_follow_path_int(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        index: Spanned<usize>,
    ) -> Result<Value, LabeledError> {
        let result = match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
            CustomValueType::NuLazyFrame(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
            CustomValueType::NuExpression(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
            CustomValueType::NuLazyGroupBy(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
            CustomValueType::NuWhen(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
        };
        Ok(result?)
    }

    fn custom_value_follow_path_string(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        column_name: Spanned<String>,
    ) -> Result<Value, LabeledError> {
        let result = match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
            CustomValueType::NuLazyFrame(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
            CustomValueType::NuExpression(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
            CustomValueType::NuLazyGroupBy(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
            CustomValueType::NuWhen(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
        };
        Ok(result?)
    }

    fn custom_value_partial_cmp(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
        other_value: Value,
    ) -> Result<Option<Ordering>, LabeledError> {
        let result = match CustomValueType::try_from_custom_value(custom_value)? {
            CustomValueType::NuDataFrame(cv) => {
                cv.custom_value_partial_cmp(self, engine, other_value)
            }
            CustomValueType::NuLazyFrame(cv) => {
                cv.custom_value_partial_cmp(self, engine, other_value)
            }
            CustomValueType::NuExpression(cv) => {
                cv.custom_value_partial_cmp(self, engine, other_value)
            }
            CustomValueType::NuLazyGroupBy(cv) => {
                cv.custom_value_partial_cmp(self, engine, other_value)
            }
            CustomValueType::NuWhen(cv) => cv.custom_value_partial_cmp(self, engine, other_value),
        };
        Ok(result?)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::values::PolarsPluginObject;
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{engine::Command, ShellError, Span};

    impl PolarsPlugin {
        /// Creates a new polars plugin in test mode
        pub fn new_test_mode() -> Self {
            PolarsPlugin {
                disable_cache_drop: true,
                ..PolarsPlugin::default()
            }
        }
    }

    pub fn test_polars_plugin_command(command: &impl PluginCommand) -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(command, vec![])
    }

    pub fn test_polars_plugin_command_with_decls(
        command: &impl PluginCommand,
        decls: Vec<Box<dyn Command>>,
    ) -> Result<(), ShellError> {
        let plugin = PolarsPlugin::new_test_mode();
        let examples = command.examples();

        // we need to cache values in the examples
        for example in &examples {
            if let Some(ref result) = example.result {
                // if it's a polars plugin object, try to cache it
                if let Ok(obj) = PolarsPluginObject::try_from_value(&plugin, result) {
                    let id = obj.id();
                    plugin
                        .cache
                        .insert(None, id, obj, Span::test_data())
                        .unwrap();
                }
            }
        }

        let mut plugin_test = PluginTest::new("polars", plugin.into())?;

        for decl in decls {
            let _ = plugin_test.add_decl(decl)?;
        }
        plugin_test.test_examples(&examples)?;

        Ok(())
    }
}
