use std::{cmp::Ordering, path::Path};

use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value, ast::Operator, casing::Casing};
use nu_utils::SharedCow;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// An opaque container for a custom value that is handled fully by a plugin.
///
/// This is the only type of custom value that is allowed to cross the plugin serialization
/// boundary.
///
/// The plugin is responsible for ensuring that local plugin custom values are converted to and from
/// [`PluginCustomValue`] on the boundary.
///
/// The engine is responsible for adding tracking the source of the custom value, ensuring that only
/// [`PluginCustomValue`] is contained within any values sent, and that the source of any values
/// sent matches the plugin it is being sent to.
///
/// Most of the [`CustomValue`] methods on this type will result in a panic. The source must be
/// added (see `nu_plugin_engine::PluginCustomValueWithSource`) in order to implement the
/// functionality via plugin calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginCustomValue(SharedCow<SharedContent>);

/// Content shared across copies of a plugin custom value.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SharedContent {
    /// The name of the type of the custom value as defined by the plugin (`type_name()`)
    name: String,
    /// The bincoded representation of the custom value on the plugin side
    data: Vec<u8>,
    /// True if the custom value should notify the source if all copies of it are dropped.
    ///
    /// This is not serialized if `false`, since most custom values don't need it.
    #[serde(default, skip_serializing_if = "is_false")]
    notify_on_drop: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

#[typetag::serde]
impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        self.name().to_owned()
    }

    fn to_base_value(&self, _span: Span) -> Result<Value, ShellError> {
        panic!("to_base_value() not available on plugin custom value without source");
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        _index: usize,
        _path_span: Span,
        _optional: bool,
    ) -> Result<Value, ShellError> {
        panic!("follow_path_int() not available on plugin custom value without source");
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        _column_name: String,
        _path_span: Span,
        _optional: bool,
        _casing: Casing,
    ) -> Result<Value, ShellError> {
        panic!("follow_path_string() not available on plugin custom value without source");
    }

    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        panic!("partial_cmp() not available on plugin custom value without source");
    }

    fn operation(
        &self,
        _lhs_span: Span,
        _operator: Operator,
        _op_span: Span,
        _right: &Value,
    ) -> Result<Value, ShellError> {
        panic!("operation() not available on plugin custom value without source");
    }

    fn save(
        &self,
        _path: Spanned<&Path>,
        _value_span: Span,
        _save_span: Span,
    ) -> Result<(), ShellError> {
        panic!("save() not available on plugin custom value without source");
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl PluginCustomValue {
    /// Create a new [`PluginCustomValue`].
    pub fn new(name: String, data: Vec<u8>, notify_on_drop: bool) -> PluginCustomValue {
        PluginCustomValue(SharedCow::new(SharedContent {
            name,
            data,
            notify_on_drop,
        }))
    }

    /// Create a [`Value`] containing this custom value.
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    /// The name of the type of the custom value as defined by the plugin (`type_name()`)
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// The bincoded representation of the custom value on the plugin side
    pub fn data(&self) -> &[u8] {
        &self.0.data
    }

    /// True if the custom value should notify the source if all copies of it are dropped.
    pub fn notify_on_drop(&self) -> bool {
        self.0.notify_on_drop
    }

    /// Count the number of shared copies of this [`PluginCustomValue`].
    pub fn ref_count(&self) -> usize {
        SharedCow::ref_count(&self.0)
    }

    /// Serialize a custom value into a [`PluginCustomValue`]. This should only be done on the
    /// plugin side.
    pub fn serialize_from_custom_value(
        custom_value: &dyn CustomValue,
        span: Span,
    ) -> Result<PluginCustomValue, ShellError> {
        let name = custom_value.type_name();
        let notify_on_drop = custom_value.notify_plugin_on_drop();
        rmp_serde::to_vec(custom_value)
            .map(|data| PluginCustomValue::new(name, data, notify_on_drop))
            .map_err(|err| ShellError::CustomValueFailedToEncode {
                msg: err.to_string(),
                span,
            })
    }

    /// Deserialize a [`PluginCustomValue`] into a `Box<dyn CustomValue>`. This should only be done
    /// on the plugin side.
    pub fn deserialize_to_custom_value(
        &self,
        span: Span,
    ) -> Result<Box<dyn CustomValue>, ShellError> {
        rmp_serde::from_slice::<Box<dyn CustomValue>>(self.data()).map_err(|err| {
            ShellError::CustomValueFailedToDecode {
                msg: err.to_string(),
                span,
            }
        })
    }
    /// Convert all plugin-native custom values to [`PluginCustomValue`] within the given `value`,
    /// recursively. This should only be done on the plugin side.
    pub fn serialize_custom_values_in(value: &mut Value) -> Result<(), ShellError> {
        value.recurse_mut(&mut |value| {
            let span = value.span();
            match value {
                Value::Custom { val, .. } => {
                    if val.as_any().downcast_ref::<PluginCustomValue>().is_some() {
                        // Already a PluginCustomValue
                        Ok(())
                    } else {
                        let serialized = Self::serialize_from_custom_value(&**val, span)?;
                        *value = Value::custom(Box::new(serialized), span);
                        Ok(())
                    }
                }
                _ => Ok(()),
            }
        })
    }

    /// Convert all [`PluginCustomValue`]s to plugin-native custom values within the given `value`,
    /// recursively. This should only be done on the plugin side.
    pub fn deserialize_custom_values_in(value: &mut Value) -> Result<(), ShellError> {
        value.recurse_mut(&mut |value| {
            let span = value.span();
            match value {
                Value::Custom { val, .. } => {
                    if let Some(val) = val.as_any().downcast_ref::<PluginCustomValue>() {
                        let deserialized = val.deserialize_to_custom_value(span)?;
                        *value = Value::custom(deserialized, span);
                        Ok(())
                    } else {
                        // Already not a PluginCustomValue
                        Ok(())
                    }
                }
                _ => Ok(()),
            }
        })
    }

    /// Render any custom values in the `Value` using `to_base_value()`
    pub fn render_to_base_value_in(value: &mut Value) -> Result<(), ShellError> {
        value.recurse_mut(&mut |value| {
            let span = value.span();
            match value {
                Value::Custom { val, .. } => {
                    *value = val.to_base_value(span)?;
                    Ok(())
                }
                _ => Ok(()),
            }
        })
    }
}
