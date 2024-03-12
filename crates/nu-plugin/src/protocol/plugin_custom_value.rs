use std::{cmp::Ordering, sync::Arc};

use nu_protocol::{ast::Operator, CustomValue, IntoSpanned, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::plugin::{PluginInterface, PluginSource};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginCustomValue {
    #[serde(flatten)]
    shared: SerdeArc<SharedContent>,

    /// Which plugin the custom value came from. This is not defined on the plugin side. The engine
    /// side is responsible for maintaining it, and it is not sent over the serialization boundary.
    #[serde(skip, default)]
    source: Option<Arc<PluginSource>>,
}

/// Content shared across copies of a plugin custom value.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SharedContent {
    /// The name of the custom value as defined by the plugin (`value_string()`)
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
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
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
            shared: SerdeArc(Arc::new(SharedContent {
                name,
                data,
                notify_on_drop,
            })),
            source,
        }
    }

    /// The name of the custom value as defined by the plugin (`value_string()`)
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
    #[cfg(test)]
    pub(crate) fn source(&self) -> &Option<Arc<PluginSource>> {
        &self.source
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

        // Envs probably should be passed here, but it's likely that the plugin is already running
        let empty_envs = std::iter::empty::<(&str, &str)>();

        source
            .persistent(span)
            .and_then(|p| p.get(|| Ok(empty_envs)))
            .map_err(wrap_err)
    }

    /// Serialize a custom value into a [`PluginCustomValue`]. This should only be done on the
    /// plugin side.
    pub(crate) fn serialize_from_custom_value(
        custom_value: &dyn CustomValue,
        span: Span,
    ) -> Result<PluginCustomValue, ShellError> {
        let name = custom_value.value_string();
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
    pub(crate) fn deserialize_to_custom_value(
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

    /// Add a [`PluginSource`] to all [`PluginCustomValue`]s within a value, recursively.
    pub(crate) fn add_source(value: &mut Value, source: &Arc<PluginSource>) {
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
            Value::Closure { ref mut val, .. } => {
                for (_, captured_value) in val.captures.iter_mut() {
                    Self::add_source(captured_value, source);
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
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        let span = value.span();
        match value {
            // Set source on custom value
            Value::CustomValue { val, .. } => {
                if let Some(custom_value) = val.as_any().downcast_ref::<PluginCustomValue>() {
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
                            span,
                            dest_plugin: source.name().to_owned(),
                            src_plugin: custom_value.source.as_ref().map(|s| s.name().to_owned()),
                        })
                    }
                } else {
                    // Only PluginCustomValues can be sent
                    Err(ShellError::CustomValueIncorrectForPlugin {
                        name: val.value_string(),
                        span,
                        dest_plugin: source.name().to_owned(),
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
            Value::Closure { ref mut val, .. } => val
                .captures
                .iter_mut()
                .try_for_each(|(_, captured_value)| Self::verify_source(captured_value, source)),
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
            Value::Closure { ref mut val, .. } => val
                .captures
                .iter_mut()
                .map(|(_, captured_value)| captured_value)
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
            Value::Closure { ref mut val, .. } => val
                .captures
                .iter_mut()
                .map(|(_, captured_value)| captured_value)
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

impl Drop for PluginCustomValue {
    fn drop(&mut self) {
        // If the custom value specifies notify_on_drop and this is the last copy, we need to let
        // the plugin know about it if we can.
        if self.source.is_some() && self.notify_on_drop() && Arc::strong_count(&self.shared) == 1 {
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

/// A serializable `Arc`, to avoid having to have the serde `rc` feature enabled.
#[derive(Clone, Debug)]
#[repr(transparent)]
struct SerdeArc<T>(Arc<T>);

impl<T> Serialize for SerdeArc<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for SerdeArc<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Arc::new).map(SerdeArc)
    }
}

impl<T> std::ops::Deref for SerdeArc<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
