use crate::{Record, ShellError, Span, Value};
use std::fmt;

/// Trait definition for a lazy record (where columns are evaluated on-demand)
/// To serialize a LazyRecord, convert it into a [`Value`] with [`to_value`](LazyRecord::to_value)
/// or into a [`Record`] with [`to_record`](LazyRecord::to_record).
pub trait LazyRecord<'a>: fmt::Debug + Send + Sync {
    /// All column names
    fn column_names(&'a self) -> Vec<&'a str>;

    /// Get the value for a specific column
    fn get_column_value(&self, column: &str) -> Result<Value, ShellError>;

    fn span(&self) -> Span;

    /// Convert this [`LazyRecord`] into a [`Record`] by evaluating all of its columns
    fn to_record(&'a self) -> Result<Record, ShellError> {
        self.column_names()
            .into_iter()
            .map(|col| {
                let val = self.get_column_value(col)?;
                Ok((col.to_owned(), val))
            })
            .collect()
    }

    /// Convert this [`LazyRecord`] into a [`Value`] by evaluating all of its columns
    fn to_value(&'a self) -> Result<Value, ShellError> {
        self.to_record()
            .map(|record| Value::record(record, self.span()))
    }

    fn clone_value(&self, span: Span) -> Value;
}
