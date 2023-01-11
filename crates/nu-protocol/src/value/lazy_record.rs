use crate::{ShellError, Span, Value};
use std::{cmp::Ordering, fmt};

// Trait definition for a lazy record (where columns are evaluated on-demand)
// typetag is needed to make this implement Serialize+Deserialize... even though we should never actually serialize a LazyRecord.
// To serialize a LazyRecord, collect it into a Value::Record first.
#[typetag::serde(tag = "type")]
pub trait LazyRecord: fmt::Debug + Send + Sync {
    // All column names
    fn column_names(&self) -> Vec<&'static str>;

    fn get_column_value(&self, column: &str) -> Result<Value, ShellError>;

    // Convert the lazy record into a regular Value::Record by collecting all its columns
    fn collect(&self) -> Result<Value, ShellError> {
        let mut cols = vec![];
        let mut vals = vec![];

        for column in self.column_names() {
            cols.push(column.into());
            let val = self.get_column_value(column)?;
            vals.push(val);
        }

        Ok(Value::Record {
            cols,
            vals,
            span: self.span(),
        })
    }

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
