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
    fn follow_path_int(&self, _count: usize, span: Span) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span,
        })
    }

    /// Follow cell path by string key (e.g. columns)
    fn follow_path_string(&self, _column_name: String, span: Span) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span,
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
        _lhs_span: Span,
        operator: Operator,
        op: Span,
        _right: &Value,
    ) -> Result<Value, ShellError> {
        Err(ShellError::UnsupportedOperator { operator, span: op })
    }
}
