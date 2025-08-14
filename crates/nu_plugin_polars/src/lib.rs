#![allow(clippy::result_large_err)]
use std::{
    cmp::Ordering,
    panic::{AssertUnwindSafe, catch_unwind},
};

use cache::cache_commands;
pub use cache::{Cache, Cacheable};
use command::{
    aggregation::aggregation_commands, boolean::boolean_commands,
    computation::computation_commands, core::core_commands, data::data_commands,
    datetime::datetime_commands, index::index_commands, integer::integer_commands,
    list::list_commands, string::string_commands, stub::PolarsCmd,
};
use log::debug;
use nu_plugin::{EngineInterface, Plugin, PluginCommand};

mod cache;
mod cloud;
pub mod dataframe;
pub use dataframe::*;
use nu_protocol::{CustomValue, LabeledError, ShellError, Span, Spanned, Value, ast::Operator};
use tokio::runtime::Runtime;
use values::CustomValueType;

use crate::values::PolarsPluginCustomValue;

pub trait EngineWrapper {
    fn get_env_var(&self, key: &str) -> Option<String>;
    fn use_color(&self) -> bool;
    fn set_gc_disabled(&self, disabled: bool) -> Result<(), ShellError>;
}

impl EngineWrapper for &EngineInterface {
    fn get_env_var(&self, key: &str) -> Option<String> {
        EngineInterface::get_env_var(self, key)
            .ok()
            .flatten()
            .map(|x| match x {
                Value::String { val, .. } => val,
                _ => "".to_string(),
            })
    }

    fn use_color(&self) -> bool {
        self.get_config()
            .ok()
            .and_then(|config| config.color_config.get("use_color").cloned())
            .unwrap_or(Value::bool(false, Span::unknown()))
            .is_true()
    }

    fn set_gc_disabled(&self, disabled: bool) -> Result<(), ShellError> {
        debug!("set_gc_disabled called with {disabled}");
        EngineInterface::set_gc_disabled(self, disabled)
    }
}

pub struct PolarsPlugin {
    pub(crate) cache: Cache,
    /// For testing purposes only
    pub(crate) disable_cache_drop: bool,
    pub(crate) runtime: Runtime,
}

impl PolarsPlugin {
    pub fn new() -> Result<Self, ShellError> {
        Ok(Self {
            cache: Cache::default(),
            disable_cache_drop: false,
            runtime: Runtime::new().map_err(|e| ShellError::GenericError {
                error: format!("Could not instantiate tokio: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })?,
        })
    }
}

impl Plugin for PolarsPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        let mut commands: Vec<Box<dyn PluginCommand<Plugin = Self>>> = vec![Box::new(PolarsCmd)];

        commands.append(&mut aggregation_commands());
        commands.append(&mut boolean_commands());
        commands.append(&mut core_commands());
        commands.append(&mut computation_commands());
        commands.append(&mut data_commands());
        commands.append(&mut datetime_commands());
        commands.append(&mut index_commands());
        commands.append(&mut integer_commands());
        commands.append(&mut string_commands());
        commands.append(&mut list_commands());

        commands.append(&mut cache_commands());
        commands
    }

    fn custom_value_dropped(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        debug!("custom_value_dropped called {:?}", custom_value);
        if !self.disable_cache_drop {
            let id = CustomValueType::try_from_custom_value(custom_value)?.id();
            let _ = self.cache.remove(engine, &id, false);
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
            CustomValueType::NuDataType(cv) => cv.custom_value_to_base_value(self, engine),
            CustomValueType::NuSchema(cv) => cv.custom_value_to_base_value(self, engine),
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
            CustomValueType::NuDataType(cv) => {
                cv.custom_value_operation(self, engine, left.span, operator, right)
            }
            CustomValueType::NuSchema(cv) => {
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
            CustomValueType::NuDataType(cv) => {
                cv.custom_value_follow_path_int(self, engine, custom_value.span, index)
            }
            CustomValueType::NuSchema(cv) => {
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
            CustomValueType::NuDataType(cv) => {
                cv.custom_value_follow_path_string(self, engine, custom_value.span, column_name)
            }
            CustomValueType::NuSchema(cv) => {
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
            CustomValueType::NuDataType(cv) => {
                cv.custom_value_partial_cmp(self, engine, other_value)
            }
            CustomValueType::NuSchema(cv) => cv.custom_value_partial_cmp(self, engine, other_value),
        };
        Ok(result?)
    }
}

pub(crate) fn handle_panic<F, R>(f: F, span: Span) -> Result<R, ShellError>
where
    F: FnOnce() -> Result<R, ShellError>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(inner_result) => inner_result,
        Err(_) => Err(ShellError::GenericError {
            error: "Panic occurred".into(),
            msg: "".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::values::PolarsPluginObject;
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{ShellError, Span, engine::Command};

    impl PolarsPlugin {
        /// Creates a new polars plugin in test mode
        pub fn new_test_mode() -> Result<Self, ShellError> {
            Ok(PolarsPlugin {
                disable_cache_drop: true,
                ..PolarsPlugin::new()?
            })
        }
    }

    struct TestEngineWrapper;

    impl EngineWrapper for TestEngineWrapper {
        fn get_env_var(&self, key: &str) -> Option<String> {
            std::env::var(key).ok()
        }

        fn use_color(&self) -> bool {
            false
        }

        fn set_gc_disabled(&self, _disabled: bool) -> Result<(), ShellError> {
            Ok(())
        }
    }

    pub fn test_polars_plugin_command(command: &impl PluginCommand) -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(command, vec![])
    }

    pub fn test_polars_plugin_command_with_decls(
        command: &impl PluginCommand,
        decls: Vec<Box<dyn Command>>,
    ) -> Result<(), ShellError> {
        let plugin = PolarsPlugin::new_test_mode()?;
        let examples = command.examples();

        // we need to cache values in the examples
        for example in &examples {
            if let Some(ref result) = example.result {
                // if it's a polars plugin object, try to cache it
                if let Ok(obj) = PolarsPluginObject::try_from_value(&plugin, result) {
                    let id = obj.id();
                    plugin
                        .cache
                        .insert(TestEngineWrapper {}, id, obj, Span::test_data())
                        .unwrap();
                }
            }
        }

        let mut plugin_test = PluginTest::new(command.name(), plugin.into())?;

        for decl in decls {
            let _ = plugin_test.add_decl(decl)?;
        }
        plugin_test.test_examples(&examples)
    }
}
