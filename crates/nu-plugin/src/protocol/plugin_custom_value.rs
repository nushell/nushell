use std::cmp::Ordering;
use std::sync::Arc;

use crate::{
    plugin::{PluginInterface, PluginSource},
    util::with_custom_values_in,
};
use nu_protocol::{ast::Operator, CustomValue, IntoSpanned, ShellError, Span, Spanned, Value};
use nu_utils::SharedCow;

use serde::{Deserialize, Serialize};

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
/// appropriate [`PluginSource`](crate::plugin::PluginSource), ensuring that only
/// [`PluginCustomData`] is contained within any values sent, and that the `source` of any
/// values sent matches the plugin it is being sent to.
///
/// This is not a public API.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[doc(hidden)]
pub struct PluginCustomValue {
    #[serde(flatten)]
    shared: SharedCow<SharedContent>,

    /// Which plugin the custom value came from. This is not defined on the plugin side. The engine
    /// side is responsible for maintaining it, and it is not sent over the serialization boundary.
    #[serde(skip, default)]
    source: Option<Arc<PluginSource>>,
}

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

impl PluginCustomValue {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

#[typetag::serde]
impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        self.name().to_owned()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        self.get_plugin(Some(span), "get base value")?
            .custom_value_to_base_value(self.clone().into_spanned(span))
    }

    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        self.get_plugin(Some(self_span), "follow cell path")?
            .custom_value_follow_path_int(
                self.clone().into_spanned(self_span),
                index.into_spanned(path_span),
            )
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        self.get_plugin(Some(self_span), "follow cell path")?
            .custom_value_follow_path_string(
                self.clone().into_spanned(self_span),
                column_name.into_spanned(path_span),
            )
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        self.get_plugin(Some(other.span()), "perform comparison")
            .and_then(|plugin| {
                // We're passing Span::unknown() here because we don't have one, and it probably
                // shouldn't matter here and is just a consequence of the API
                plugin.custom_value_partial_cmp(self.clone(), other.clone())
            })
            .unwrap_or_else(|err| {
                // We can't do anything with the error other than log it.
                log::warn!(
                    "Error in partial_cmp on plugin custom value (source={source:?}): {err}",
                    source = self.source
                );
                None
            })
            .map(|ordering| ordering.into())
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op_span: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        self.get_plugin(Some(lhs_span), "invoke operator")?
            .custom_value_operation(
                self.clone().into_spanned(lhs_span),
                operator.into_spanned(op_span),
                right.clone(),
            )
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
    pub(crate) fn new(
        name: String,
        data: Vec<u8>,
        notify_on_drop: bool,
        source: Option<Arc<PluginSource>>,
    ) -> PluginCustomValue {
        PluginCustomValue {
            shared: SharedCow::new(SharedContent {
                name,
                data,
                notify_on_drop,
            }),
            source,
        }
    }

    /// The name of the type of the custom value as defined by the plugin (`type_name()`)
    pub fn name(&self) -> &str {
        &self.shared.name
    }

    /// The bincoded representation of the custom value on the plugin side
    pub fn data(&self) -> &[u8] {
        &self.shared.data
    }

    /// True if the custom value should notify the source if all copies of it are dropped.
    pub fn notify_on_drop(&self) -> bool {
        self.shared.notify_on_drop
    }

    /// Which plugin the custom value came from. This is not defined on the plugin side. The engine
    /// side is responsible for maintaining it, and it is not sent over the serialization boundary.
    pub fn source(&self) -> &Option<Arc<PluginSource>> {
        &self.source
    }

    /// Set the [`PluginSource`] for this [`PluginCustomValue`].
    pub fn set_source(&mut self, source: Option<Arc<PluginSource>>) {
        self.source = source;
    }

    /// Create the [`PluginCustomValue`] with the given source.
    #[cfg(test)]
    pub(crate) fn with_source(mut self, source: Option<Arc<PluginSource>>) -> PluginCustomValue {
        self.source = source;
        self
    }

    /// Helper to get the plugin to implement an op
    fn get_plugin(&self, span: Option<Span>, for_op: &str) -> Result<PluginInterface, ShellError> {
        let wrap_err = |err: ShellError| ShellError::GenericError {
            error: format!(
                "Unable to spawn plugin `{}` to {for_op}",
                self.source
                    .as_ref()
                    .map(|s| s.name())
                    .unwrap_or("<unknown>")
            ),
            msg: err.to_string(),
            span,
            help: None,
            inner: vec![err],
        };

        let source = self.source.clone().ok_or_else(|| {
            wrap_err(ShellError::NushellFailed {
                msg: "The plugin source for the custom value was not set".into(),
            })
        })?;

        source
            .persistent(span)
            .and_then(|p| p.get_plugin(None))
            .map_err(wrap_err)
    }

    /// Serialize a custom value into a [`PluginCustomValue`]. This should only be done on the
    /// plugin side.
    pub fn serialize_from_custom_value(
        custom_value: &dyn CustomValue,
        span: Span,
    ) -> Result<PluginCustomValue, ShellError> {
        let name = custom_value.type_name();
        let notify_on_drop = custom_value.notify_plugin_on_drop();
        bincode::serialize(custom_value)
            .map(|data| PluginCustomValue::new(name, data, notify_on_drop, None))
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
        bincode::deserialize::<Box<dyn CustomValue>>(self.data()).map_err(|err| {
            ShellError::CustomValueFailedToDecode {
                msg: err.to_string(),
                span,
            }
        })
    }

    /// Add a [`PluginSource`] to the given [`CustomValue`] if it is a [`PluginCustomValue`].
    pub fn add_source(value: &mut dyn CustomValue, source: &Arc<PluginSource>) {
        if let Some(custom_value) = value.as_mut_any().downcast_mut::<PluginCustomValue>() {
            custom_value.set_source(Some(source.clone()));
        }
    }

    /// Add a [`PluginSource`] to all [`PluginCustomValue`]s within the value, recursively.
    pub fn add_source_in(value: &mut Value, source: &Arc<PluginSource>) -> Result<(), ShellError> {
        with_custom_values_in(value, |custom_value| {
            Self::add_source(custom_value.item, source);
            Ok::<_, ShellError>(())
        })
    }

    /// Check that a [`CustomValue`] is a [`PluginCustomValue`] that come from the given `source`,
    /// and return an error if not.
    ///
    /// This method will collapse `LazyRecord` in-place as necessary to make the guarantee,
    /// since `LazyRecord` could return something different the next time it is called.
    pub fn verify_source(
        value: Spanned<&dyn CustomValue>,
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        if let Some(custom_value) = value.item.as_any().downcast_ref::<PluginCustomValue>() {
            if custom_value
                .source
                .as_ref()
                .map(|s| s.is_compatible(source))
                .unwrap_or(false)
            {
                Ok(())
            } else {
                Err(ShellError::CustomValueIncorrectForPlugin {
                    name: custom_value.name().to_owned(),
                    span: value.span,
                    dest_plugin: source.name().to_owned(),
                    src_plugin: custom_value.source.as_ref().map(|s| s.name().to_owned()),
                })
            }
        } else {
            // Only PluginCustomValues can be sent
            Err(ShellError::CustomValueIncorrectForPlugin {
                name: value.item.type_name(),
                span: value.span,
                dest_plugin: source.name().to_owned(),
                src_plugin: None,
            })
        }
    }

    /// Convert all plugin-native custom values to [`PluginCustomValue`] within the given `value`,
    /// recursively. This should only be done on the plugin side.
    pub fn serialize_custom_values_in(value: &mut Value) -> Result<(), ShellError> {
        value.recurse_mut(&mut |value| {
            let span = value.span();
            match value {
                Value::Custom { ref val, .. } => {
                    if val.as_any().downcast_ref::<PluginCustomValue>().is_some() {
                        // Already a PluginCustomValue
                        Ok(())
                    } else {
                        let serialized = Self::serialize_from_custom_value(&**val, span)?;
                        *value = Value::custom(Box::new(serialized), span);
                        Ok(())
                    }
                }
                // Collect LazyRecord before proceeding
                Value::LazyRecord { ref val, .. } => {
                    *value = val.collect()?;
                    Ok(())
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
                Value::Custom { ref val, .. } => {
                    if let Some(val) = val.as_any().downcast_ref::<PluginCustomValue>() {
                        let deserialized = val.deserialize_to_custom_value(span)?;
                        *value = Value::custom(deserialized, span);
                        Ok(())
                    } else {
                        // Already not a PluginCustomValue
                        Ok(())
                    }
                }
                // Collect LazyRecord before proceeding
                Value::LazyRecord { ref val, .. } => {
                    *value = val.collect()?;
                    Ok(())
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
                Value::Custom { ref val, .. } => {
                    *value = val.to_base_value(span)?;
                    Ok(())
                }
                // Collect LazyRecord before proceeding
                Value::LazyRecord { ref val, .. } => {
                    *value = val.collect()?;
                    Ok(())
                }
                _ => Ok(()),
            }
        })
    }
}

impl Drop for PluginCustomValue {
    fn drop(&mut self) {
        // If the custom value specifies notify_on_drop and this is the last copy, we need to let
        // the plugin know about it if we can.
        if self.source.is_some() && self.notify_on_drop() && SharedCow::ref_count(&self.shared) == 1
        {
            self.get_plugin(None, "drop")
                // While notifying drop, we don't need a copy of the source
                .and_then(|plugin| {
                    plugin.custom_value_dropped(PluginCustomValue {
                        shared: self.shared.clone(),
                        source: None,
                    })
                })
                .unwrap_or_else(|err| {
                    // We shouldn't do anything with the error except log it
                    let name = self.name();
                    log::warn!("Failed to notify drop of custom value ({name}): {err}")
                });
        }
    }
}
