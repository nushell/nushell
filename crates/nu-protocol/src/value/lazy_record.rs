use std::{cmp::Ordering, collections::HashMap, fmt};

use serde::{Deserialize, Serialize, Serializer};

use crate::{ast::PathMember, ShellError, Span, Value};

// Trait definition for a lazy record (where columns are evaluated on-demand)
// really not sure about this...
#[typetag::serde(tag = "type")]
pub trait LazyRecord: fmt::Debug + Send + Sync {
    // fn clone_value(&self, span: Span) -> Value;

    //fn category(&self) -> Category;

    // String representation of the custom value
    // TODO is this right?
    fn value_string(&self) -> String;

    fn get_column_map(
        &self,
        span: Span,
    ) -> HashMap<String, Box<dyn Fn() -> Result<Value, ShellError>>>;

    // Converts the custom value to a base nushell value
    // This is used to represent the custom value using the table representations
    // That already exist in nushell
    fn collect(&self, span: Span) -> Result<Value, ShellError> {
        let map = self.get_column_map(span);

        let mut cols = vec![];
        let mut vals = vec![];

        for (column_name, closure) in map.iter() {
            cols.push(column_name.clone());
            let value = closure()?;
            vals.push(value);
        }

        Ok(Value::Record { cols, vals, span })
    }

    // fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    //     todo!()
    // }

    // Any representation used to downcast object to its original type
    // fn as_any(&self) -> &dyn std::any::Any;

    fn get_column_value(&self, column: &String, span: Span) -> Result<Value, ShellError> {
        let hashmap = self.get_column_map(span);
        if let Some(closure) = hashmap.get(column) {
            closure()
        } else {
            Err(ShellError::CantFindColumn(
                column.to_string(),
                span,
                span,
            ))
        }
    }

    // fn follow_cell_path(&self, cell_path: &[PathMember], span: Span) -> Result<Value, ShellError> {
    //     // TODO: get insensitive from config?
    //     self.collect(span)?.follow_cell_path(cell_path, true)
    // }

    // ordering with other value
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }
}
