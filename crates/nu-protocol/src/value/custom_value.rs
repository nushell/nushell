use std::{cmp::Ordering, fmt};

use crate::{ast::Operator, ShellError, Span, Value};

/// Trait definition for a custom [`Value`](crate::Value) type
#[typetag::serde(tag = "type")]
pub trait CustomValue: fmt::Debug + Send + Sync {
    /// Custom `Clone` implementation
    ///
    /// This can reemit a `Value::CustomValue(Self, span)` or materialize another representation
    /// if necessary.
    fn clone_value(&self, span: Span) -> Value;

    //fn category(&self) -> Category;

    /// Define string representation of the custom value
    fn value_string(&self) -> String;

    /// Converts the custom value to a base nushell value.
    ///
    /// This imposes the requirement that you can represent the custom value in some form using the
    /// Value representations that already exist in nushell
    fn to_base_value(&self, span: Span) -> Result<Value, ShellError>;

    /// Any representation used to downcast object to its original type
    fn as_any(&self) -> &dyn std::any::Any;

    /// Follow cell path by numeric index (e.g. rows)
    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, index);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span: path_span,
        })
    }

    /// Follow cell path by string key (e.g. columns)
    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, column_name);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span: path_span,
        })
    }

    /// ordering with other value (see [`std::cmp::PartialOrd`])
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }

    /// Definition of an operation between the object that implements the trait
    /// and another Value.
    ///
    /// The Operator enum is used to indicate the expected operation.
    ///
    /// Default impl raises [`ShellError::UnsupportedOperator`].
    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let _ = (lhs_span, right);
        Err(ShellError::UnsupportedOperator { operator, span: op })
    }

    /// For custom values in plugins: return `true` here if you would like to be notified when all
    /// copies of this custom value are dropped in the engine.
    ///
    /// The notification will take place via
    /// [`.custom_value_dropped()`](crate::StreamingPlugin::custom_value_dropped) on the plugin.
    ///
    /// The default is `false`.
    fn notify_plugin_on_drop(&self) -> bool {
        false
    }
}
