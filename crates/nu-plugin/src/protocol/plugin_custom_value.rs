use std::sync::Arc;

use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value};
use serde::{Deserialize, Serialize};

use crate::plugin::PluginIdentity;

#[cfg(test)]
mod tests;

/// An opaque container for a custom value that is handled fully by a plugin
///
/// This is the only type of custom value that is allowed to cross the plugin serialization
/// boundary.
///
/// [`EngineInterface`](crate::interface::EngineInterface) is responsible for ensuring
/// that local plugin custom values are converted to and from [`PluginCustomData`] on the boundary.
///
/// [`PluginInterface`](crate::interface::PluginInterface) is responsible for adding the
/// appropriate [`PluginIdentity`](crate::plugin::PluginIdentity), ensuring that only
/// [`PluginCustomData`] is contained within any values sent, and that the `source` of any
/// values sent matches the plugin it is being sent to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginCustomValue {
    /// The name of the custom value as defined by the plugin (`value_string()`)
    pub name: String,
    /// The bincoded representation of the custom value on the plugin side
    pub data: Vec<u8>,

    /// Which plugin the custom value came from. This is not defined on the plugin side. The engine
    /// side is responsible for maintaining it, and it is not sent over the serialization boundary.
    #[serde(skip, default)]
    pub source: Option<Arc<PluginIdentity>>,
}

#[typetag::serde]
impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> nu_protocol::Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        self.name.clone()
    }

    fn to_base_value(
        &self,
        span: nu_protocol::Span,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let wrap_err = |err: ShellError| ShellError::GenericError {
            error: format!(
                "Unable to spawn plugin `{}` to get base value",
                self.source
                    .as_ref()
                    .map(|s| s.plugin_name.as_str())
                    .unwrap_or("<unknown>")
            ),
            msg: err.to_string(),
            span: Some(span),
            help: None,
            inner: vec![err],
        };

        let identity = self.source.clone().ok_or_else(|| {
            wrap_err(ShellError::NushellFailed {
                msg: "The plugin source for the custom value was not set".into(),
            })
        })?;

        let empty_env: Option<(String, String)> = None;
        let plugin = identity.spawn(empty_env).map_err(wrap_err)?;

        plugin
            .custom_value_to_base_value(Spanned {
                item: self.clone(),
                span,
            })
            .map_err(wrap_err)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl PluginCustomValue {
    /// Serialize a custom value into a [`PluginCustomValue`]. This should only be done on the
    /// plugin side.
    pub(crate) fn serialize_from_custom_value(
        custom_value: &dyn CustomValue,
        span: Span,
    ) -> Result<PluginCustomValue, ShellError> {
        let name = custom_value.value_string();
        bincode::serialize(custom_value)
            .map(|data| PluginCustomValue {
                name,
                data,
                source: None,
            })
            .map_err(|err| ShellError::CustomValueFailedToEncode {
                msg: err.to_string(),
                span,
            })
    }

    /// Deserialize a [`PluginCustomValue`] into a `Box<dyn CustomValue>`. This should only be done
    /// on the plugin side.
    pub(crate) fn deserialize_to_custom_value(
        &self,
        span: Span,
    ) -> Result<Box<dyn CustomValue>, ShellError> {
        bincode::deserialize::<Box<dyn CustomValue>>(&self.data).map_err(|err| {
            ShellError::CustomValueFailedToDecode {
                msg: err.to_string(),
                span,
            }
        })
    }

    /// Add a [`PluginIdentity`] to all [`PluginCustomValue`]s within a value, recursively.
    pub(crate) fn add_source(value: &mut Value, source: &Arc<PluginIdentity>) {
        let span = value.span();
        match value {
            // Set source on custom value
            Value::CustomValue { ref val, .. } => {
                if let Some(custom_value) = val.as_any().downcast_ref::<PluginCustomValue>() {
                    // Since there's no `as_mut_any()`, we have to copy the whole thing
                    let mut custom_value = custom_value.clone();
                    custom_value.source = Some(source.clone());
                    *value = Value::custom_value(Box::new(custom_value), span);
                }
            }
            // Any values that can contain other values need to be handled recursively
            Value::Range { ref mut val, .. } => {
                Self::add_source(&mut val.from, source);
                Self::add_source(&mut val.to, source);
                Self::add_source(&mut val.incr, source);
            }
            Value::Record { ref mut val, .. } => {
                for (_, rec_value) in val.iter_mut() {
                    Self::add_source(rec_value, source);
                }
            }
            Value::List { ref mut vals, .. } => {
                for list_value in vals.iter_mut() {
                    Self::add_source(list_value, source);
                }
            }
            // All of these don't contain other values
            Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Block { .. }
            | Value::Closure { .. }
            | Value::Nothing { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. } => (),
            // LazyRecord could generate other values, but we shouldn't be receiving it anyway
            //
            // It's better to handle this as a bug
            Value::LazyRecord { .. } => unimplemented!("add_source for LazyRecord"),
        }
    }

    /// Check that all [`CustomValue`]s present within the `value` are [`PluginCustomValue`]s that
    /// come from the given `source`, and return an error if not.
    ///
    /// This method will collapse `LazyRecord` in-place as necessary to make the guarantee,
    /// since `LazyRecord` could return something different the next time it is called.
    pub(crate) fn verify_source(
        value: &mut Value,
        source: &PluginIdentity,
    ) -> Result<(), ShellError> {
        let span = value.span();
        match value {
            // Set source on custom value
            Value::CustomValue { val, .. } => {
                if let Some(custom_value) = val.as_any().downcast_ref::<PluginCustomValue>() {
                    if custom_value.source.as_deref() == Some(source) {
                        Ok(())
                    } else {
                        Err(ShellError::CustomValueIncorrectForPlugin {
                            name: custom_value.name.clone(),
                            span,
                            dest_plugin: source.plugin_name.clone(),
                            src_plugin: custom_value.source.as_ref().map(|s| s.plugin_name.clone()),
                        })
                    }
                } else {
                    // Only PluginCustomValues can be sent
                    Err(ShellError::CustomValueIncorrectForPlugin {
                        name: val.value_string(),
                        span,
                        dest_plugin: source.plugin_name.clone(),
                        src_plugin: None,
                    })
                }
            }
            // Any values that can contain other values need to be handled recursively
            Value::Range { val, .. } => {
                Self::verify_source(&mut val.from, source)?;
                Self::verify_source(&mut val.to, source)?;
                Self::verify_source(&mut val.incr, source)
            }
            Value::Record { ref mut val, .. } => val
                .iter_mut()
                .try_for_each(|(_, rec_value)| Self::verify_source(rec_value, source)),
            Value::List { ref mut vals, .. } => vals
                .iter_mut()
                .try_for_each(|list_value| Self::verify_source(list_value, source)),
            // All of these don't contain other values
            Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Block { .. }
            | Value::Closure { .. }
            | Value::Nothing { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. } => Ok(()),
            // LazyRecord would be a problem for us, since it could return something else the next
            // time, and we have to collect it anyway to serialize it. Collect it in place, and then
            // verify the source of the result
            Value::LazyRecord { val, .. } => {
                *value = val.collect()?;
                Self::verify_source(value, source)
            }
        }
    }

    /// Convert all plugin-native custom values to [`PluginCustomValue`] within the given `value`,
    /// recursively. This should only be done on the plugin side.
    pub(crate) fn serialize_custom_values_in(value: &mut Value) -> Result<(), ShellError> {
        let span = value.span();
        match value {
            Value::CustomValue { ref val, .. } => {
                if val.as_any().downcast_ref::<PluginCustomValue>().is_some() {
                    // Already a PluginCustomValue
                    Ok(())
                } else {
                    let serialized = Self::serialize_from_custom_value(&**val, span)?;
                    *value = Value::custom_value(Box::new(serialized), span);
                    Ok(())
                }
            }
            // Any values that can contain other values need to be handled recursively
            Value::Range { ref mut val, .. } => {
                Self::serialize_custom_values_in(&mut val.from)?;
                Self::serialize_custom_values_in(&mut val.to)?;
                Self::serialize_custom_values_in(&mut val.incr)
            }
            Value::Record { ref mut val, .. } => val
                .iter_mut()
                .try_for_each(|(_, rec_value)| Self::serialize_custom_values_in(rec_value)),
            Value::List { ref mut vals, .. } => vals
                .iter_mut()
                .try_for_each(Self::serialize_custom_values_in),
            // All of these don't contain other values
            Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Block { .. }
            | Value::Closure { .. }
            | Value::Nothing { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. } => Ok(()),
            // Collect any lazy records that exist and try again
            Value::LazyRecord { val, .. } => {
                *value = val.collect()?;
                Self::serialize_custom_values_in(value)
            }
        }
    }

    /// Convert all [`PluginCustomValue`]s to plugin-native custom values within the given `value`,
    /// recursively. This should only be done on the plugin side.
    pub(crate) fn deserialize_custom_values_in(value: &mut Value) -> Result<(), ShellError> {
        let span = value.span();
        match value {
            Value::CustomValue { ref val, .. } => {
                if let Some(val) = val.as_any().downcast_ref::<PluginCustomValue>() {
                    let deserialized = val.deserialize_to_custom_value(span)?;
                    *value = Value::custom_value(deserialized, span);
                    Ok(())
                } else {
                    // Already not a PluginCustomValue
                    Ok(())
                }
            }
            // Any values that can contain other values need to be handled recursively
            Value::Range { ref mut val, .. } => {
                Self::deserialize_custom_values_in(&mut val.from)?;
                Self::deserialize_custom_values_in(&mut val.to)?;
                Self::deserialize_custom_values_in(&mut val.incr)
            }
            Value::Record { ref mut val, .. } => val
                .iter_mut()
                .try_for_each(|(_, rec_value)| Self::deserialize_custom_values_in(rec_value)),
            Value::List { ref mut vals, .. } => vals
                .iter_mut()
                .try_for_each(Self::deserialize_custom_values_in),
            // All of these don't contain other values
            Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Block { .. }
            | Value::Closure { .. }
            | Value::Nothing { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. } => Ok(()),
            // Collect any lazy records that exist and try again
            Value::LazyRecord { val, .. } => {
                *value = val.collect()?;
                Self::deserialize_custom_values_in(value)
            }
        }
    }
}
