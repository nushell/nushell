use crate::{ShellError, Span, Value};
use std::{cmp::Ordering, fmt};

// Trait definition for a lazy record (where columns are evaluated on-demand)
// typetag is needed to make this implement Serialize+Deserialize... even though we should never actually serialize a LazyRecord.
// To serialize a LazyRecord, collect it into a Value::Record first.
#[typetag::serde(tag = "type")]
pub trait LazyRecord: fmt::Debug + Send + Sync {
    // All column names
    fn columns(&self) -> Vec<String>;

    // Convert the lazy record into a regular Value::Record by collecting all its columns
    // This is used to represent the custom value using the table representations
    // That already exist in nushell
    fn collect(&self) -> Result<Value, ShellError>;

    fn get_column_value(&self, column: &String) -> Result<Value, ShellError>;

    fn span(&self) -> Span;

    // String representation of the lazy record.
    fn value_string(&self) -> String {
        "LazyRecord".into()
    }
    // ordering with other value
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }
}
