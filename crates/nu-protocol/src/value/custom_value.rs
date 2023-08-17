use std::{cmp::Ordering, fmt};

use crate::{ast::Operator, ShellError, Span, SpannedValue};

// Trait definition for a custom value
#[typetag::serde(tag = "type")]
pub trait CustomValue: fmt::Debug + Send + Sync {
    fn clone_value(&self, span: Span) -> SpannedValue;

    //fn category(&self) -> Category;

    // Define string representation of the custom value
    fn value_string(&self) -> String;

    // Converts the custom value to a base nushell value
    // This is used to represent the custom value using the table representations
    // That already exist in nushell
    fn to_base_value(&self, span: Span) -> Result<SpannedValue, ShellError>;

    // Any representation used to downcast object to its original type
    fn as_any(&self) -> &dyn std::any::Any;

    // Follow cell path functions
    fn follow_path_int(&self, _count: usize, span: Span) -> Result<SpannedValue, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span,
        })
    }

    fn follow_path_string(
        &self,
        _column_name: String,
        span: Span,
    ) -> Result<SpannedValue, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.value_string(),
            span,
        })
    }

    // ordering with other value
    fn partial_cmp(&self, _other: &SpannedValue) -> Option<Ordering> {
        None
    }

    // Definition of an operation between the object that implements the trait
    // and another Value.
    // The Operator enum is used to indicate the expected operation
    fn operation(
        &self,
        _lhs_span: Span,
        operator: Operator,
        op: Span,
        _right: &SpannedValue,
    ) -> Result<SpannedValue, ShellError> {
        Err(ShellError::UnsupportedOperator { operator, span: op })
    }
}
