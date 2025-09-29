use std::{cmp::Ordering, path::Path, sync::Arc};

use nu_plugin_core::util::with_custom_values_in;
use nu_plugin_protocol::PluginCustomValue;
use nu_protocol::{
    CustomValue, IntoSpanned, ShellError, Span, Spanned, Value, ast::Operator, casing::Casing,
};
use serde::Serialize;

use crate::{PluginInterface, PluginSource};

#[cfg(test)]
mod tests;

/// Wraps a [`PluginCustomValue`] together with its [`PluginSource`], so that the [`CustomValue`]
/// methods can be implemented by calling the plugin, and to ensure that any custom values sent to a
/// plugin came from it originally.
#[derive(Debug, Clone)]
pub struct PluginCustomValueWithSource {
    inner: PluginCustomValue,

    /// Which plugin the custom value came from. This is not sent over the serialization boundary.
    source: Arc<PluginSource>,
}

impl PluginCustomValueWithSource {
    /// Wrap a [`PluginCustomValue`] together with its source.
    pub fn new(inner: PluginCustomValue, source: Arc<PluginSource>) -> PluginCustomValueWithSource {
        PluginCustomValueWithSource { inner, source }
    }

    /// Create a [`Value`] containing this custom value.
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    /// Which plugin the custom value came from. This provides a direct reference to be able to get
    /// a plugin interface in order to make a call, when needed.
    pub fn source(&self) -> &Arc<PluginSource> {
        &self.source
    }

    /// Unwrap the [`PluginCustomValueWithSource`], discarding the source.
    pub fn without_source(self) -> PluginCustomValue {
        // Because of the `Drop` implementation, we can't destructure this.
        self.inner.clone()
    }

    /// Helper to get the plugin to implement an op
    fn get_plugin(&self, span: Option<Span>, for_op: &str) -> Result<PluginInterface, ShellError> {
        let wrap_err = |err: ShellError| ShellError::GenericError {
            error: format!(
                "Unable to spawn plugin `{}` to {for_op}",
                self.source.name()
            ),
            msg: err.to_string(),
            span,
            help: None,
            inner: vec![err],
        };

        self.source
            .clone()
            .persistent(span)
            .and_then(|p| p.get_plugin(None))
            .map_err(wrap_err)
    }

    /// Add a [`PluginSource`] to the given [`CustomValue`] if it is a [`PluginCustomValue`].
    pub fn add_source(value: &mut Box<dyn CustomValue>, source: &Arc<PluginSource>) {
        if let Some(custom_value) = value.as_any().downcast_ref::<PluginCustomValue>() {
            *value = Box::new(custom_value.clone().with_source(source.clone()));
        }
    }

    /// Add a [`PluginSource`] to all [`PluginCustomValue`]s within the value, recursively.
    pub fn add_source_in(value: &mut Value, source: &Arc<PluginSource>) -> Result<(), ShellError> {
        with_custom_values_in(value, |custom_value| {
            Self::add_source(custom_value.item, source);
            Ok::<_, ShellError>(())
        })
    }

    /// Remove a [`PluginSource`] from the given [`CustomValue`] if it is a
    /// [`PluginCustomValueWithSource`]. This will turn it back into a [`PluginCustomValue`].
    pub fn remove_source(value: &mut Box<dyn CustomValue>) {
        if let Some(custom_value) = value.as_any().downcast_ref::<PluginCustomValueWithSource>() {
            *value = Box::new(custom_value.clone().without_source());
        }
    }

    /// Remove the [`PluginSource`] from all [`PluginCustomValue`]s within the value, recursively.
    pub fn remove_source_in(value: &mut Value) -> Result<(), ShellError> {
        with_custom_values_in(value, |custom_value| {
            Self::remove_source(custom_value.item);
            Ok::<_, ShellError>(())
        })
    }

    /// Check that `self` came from the given `source`, and return an `error` if not.
    pub fn verify_source(&self, span: Span, source: &PluginSource) -> Result<(), ShellError> {
        if self.source.is_compatible(source) {
            Ok(())
        } else {
            Err(ShellError::CustomValueIncorrectForPlugin {
                name: self.name().to_owned(),
                span,
                dest_plugin: source.name().to_owned(),
                src_plugin: Some(self.source.name().to_owned()),
            })
        }
    }

    /// Check that a [`CustomValue`] is a [`PluginCustomValueWithSource`] that came from the given
    /// `source`, and return an error if not.
    pub fn verify_source_of_custom_value(
        value: Spanned<&dyn CustomValue>,
        source: &PluginSource,
    ) -> Result<(), ShellError> {
        if let Some(custom_value) = value
            .item
            .as_any()
            .downcast_ref::<PluginCustomValueWithSource>()
        {
            custom_value.verify_source(value.span, source)
        } else {
            // Only PluginCustomValueWithSource can be sent
            Err(ShellError::CustomValueIncorrectForPlugin {
                name: value.item.type_name(),
                span: value.span,
                dest_plugin: source.name().to_owned(),
                src_plugin: None,
            })
        }
    }
}

impl std::ops::Deref for PluginCustomValueWithSource {
    type Target = PluginCustomValue;

    fn deref(&self) -> &PluginCustomValue {
        &self.inner
    }
}

/// This `Serialize` implementation always produces an error. Strip the source before sending.
impl Serialize for PluginCustomValueWithSource {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;
        Err(Error::custom(
            "can't serialize PluginCustomValueWithSource, remove the source first",
        ))
    }
}

impl CustomValue for PluginCustomValueWithSource {
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
        optional: bool,
    ) -> Result<Value, ShellError> {
        self.get_plugin(Some(self_span), "follow cell path")?
            .custom_value_follow_path_int(
                self.clone().into_spanned(self_span),
                index.into_spanned(path_span),
                optional,
            )
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        self.get_plugin(Some(self_span), "follow cell path")?
            .custom_value_follow_path_string(
                self.clone().into_spanned(self_span),
                column_name.into_spanned(path_span),
                optional,
                casing,
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

    fn save(
        &self,
        path: Spanned<&Path>,
        value_span: Span,
        save_span: Span,
    ) -> Result<(), ShellError> {
        self.get_plugin(Some(value_span), "save")?
            .custom_value_save(self.clone().into_spanned(value_span), path, save_span)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[doc(hidden)]
    fn typetag_name(&self) -> &'static str {
        "PluginCustomValueWithSource"
    }

    #[doc(hidden)]
    fn typetag_deserialize(&self) {}
}

impl Drop for PluginCustomValueWithSource {
    fn drop(&mut self) {
        // If the custom value specifies notify_on_drop and this is the last copy, we need to let
        // the plugin know about it if we can.
        if self.notify_on_drop() && self.inner.ref_count() == 1 {
            self.get_plugin(None, "drop")
                // While notifying drop, we don't need a copy of the source
                .and_then(|plugin| plugin.custom_value_dropped(self.inner.clone()))
                .unwrap_or_else(|err| {
                    // We shouldn't do anything with the error except log it
                    let name = self.name();
                    log::warn!("Failed to notify drop of custom value ({name}): {err}")
                });
        }
    }
}

/// Helper trait for adding a source to a [`PluginCustomValue`]
pub trait WithSource {
    /// Add a source to a plugin custom value
    fn with_source(self, source: Arc<PluginSource>) -> PluginCustomValueWithSource;
}

impl WithSource for PluginCustomValue {
    fn with_source(self, source: Arc<PluginSource>) -> PluginCustomValueWithSource {
        PluginCustomValueWithSource::new(self, source)
    }
}
