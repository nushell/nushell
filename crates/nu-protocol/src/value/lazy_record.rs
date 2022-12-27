use crate::{ShellError, Span, Value};
use std::{cmp::Ordering, collections::HashMap, fmt};

// Trait definition for a lazy record (where columns are evaluated on-demand)
// typetag is needed to make this implement Serialize+Deserialize... even though we should never actually serialize a LazyRecord.
// To serialize a LazyRecord, collect it into a Value::Record first.
#[typetag::serde(tag = "type")]
pub trait LazyRecord: fmt::Debug + Send + Sync {
    // Get a map of
    fn get_column_map(&self) -> HashMap<String, Box<dyn Fn() -> Result<Value, ShellError> + '_>>;

    // Convert the lazy record into a regular Value::Record by collecting all its columns
    // This is used to represent the custom value using the table representations
    // That already exist in nushell
    fn collect(&self) -> Result<Value, ShellError> {
        let map = self.get_column_map();

        let mut cols = vec![];
        let mut vals = vec![];

        for (column_name, closure) in map.iter() {
            cols.push(column_name.clone());
            let value = closure()?;
            vals.push(value);
        }

        Ok(Value::Record {
            cols,
            vals,
            span: self.span(),
        })
    }

    fn get_column_value(&self, column: &String, span: Span) -> Result<Value, ShellError> {
        let hashmap = self.get_column_map();
        if let Some(closure) = hashmap.get(column) {
            closure()
        } else {
            Err(ShellError::CantFindColumn(
                column.to_string(),
                span,
                self.span(),
            ))
        }
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
