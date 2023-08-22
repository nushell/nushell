use crate::{Record, ShellError, Span, Value};
use std::fmt;

// Trait definition for a lazy record (where columns are evaluated on-demand)
// typetag is needed to make this implement Serialize+Deserialize... even though we should never actually serialize a LazyRecord.
// To serialize a LazyRecord, collect it into a Value::Record with collect() first.
pub trait LazyRecord<'a>: fmt::Debug + Send + Sync {
    // All column names
    fn column_names(&'a self) -> Vec<&'a str>;

    // Get 1 specific column value
    fn get_column_value(&self, column: &str) -> Result<Value, ShellError>;

    fn span(&self) -> Span;

    // Convert the lazy record into a regular Value::Record by collecting all its columns
    fn collect(&'a self) -> Result<Value, ShellError> {
        self.column_names()
            .into_iter()
            .map(|col| {
                let val = self.get_column_value(col)?;
                Ok((col.to_owned(), val))
            })
            .collect::<Result<Record, _>>()
            .map(|record| Value::record(record, self.span()))
    }

    fn clone_value(&self, span: Span) -> Value;
}
