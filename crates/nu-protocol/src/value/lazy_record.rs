use crate::{ShellError, Span, Value};
use std::fmt;

// Trait definition for a lazy record (where columns are evaluated on-demand)
// typetag is needed to make this implement Serialize+Deserialize... even though we should never actually serialize a LazyRecord.
// To serialize a LazyRecord, collect it into a Value::Record with collect() first.
#[typetag::serde(tag = "type")]
pub trait LazyRecord: fmt::Debug + Send + Sync {
    // All column names
    fn column_names(&self) -> Vec<&'static str>;

    // Get 1 specific column value
    fn get_column_value(&self, column: &str) -> Result<Value, ShellError>;

    fn span(&self) -> Span;

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
}
