use std::cmp::Ordering;

pub use cache::{Cache, Cacheable};
use dataframe::{stub::PolarsCmd, values::CustomValueType};
use nu_plugin::{EngineInterface, Plugin, PluginCommand};

mod cache;
pub mod dataframe;
pub use dataframe::*;
use nu_protocol::{ast::Operator, CustomValue, LabeledError, Spanned, Value};

use crate::{
    eager::eager_commands, expressions::expr_commands, lazy::lazy_commands,
    values::PolarsPluginCustomValue,
};

#[derive(Default)]
pub struct PolarsPlugin {
    pub(crate) cache: Cache,
}

impl Plugin for PolarsPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        let mut commands: Vec<Box<dyn PluginCommand<Plugin = Self>>> = vec![Box::new(PolarsCmd)];
        commands.append(&mut eager_commands());
        commands.append(&mut lazy_commands());
        commands.append(&mut expr_commands());
        commands
    }

    fn custom_value_dropped(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        let id = CustomValueType::try_from_custom_value(custom_value)?.id();
        let _ = self.cache.remove(engine, &id);
        Ok(())
    }

    fn custom_value_to_base_value(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
    ) -> Result<Value, LabeledError> {
        eprintln!("Polars plugin customn_value_to_base_value called");
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
