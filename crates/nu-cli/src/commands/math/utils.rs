use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, ReturnSuccess, UntaggedValue, Value};

use indexmap::map::IndexMap;

pub type MathFunction = fn(values: &[Value], tag: &Tag) -> Result<Value, ShellError>;

pub async fn run_with_function(
    RunnableContext {
        mut input, name, ..
    }: RunnableContext,
    mf: MathFunction,
) -> Result<OutputStream, ShellError> {
    let values: Vec<Value> = input.drain_vec().await;
    let res = calculate(&values, &name, mf);
    match res {
        Ok(v) => {
            if v.value.is_table() {
                Ok(OutputStream::from(
                    v.table_entries()
                        .map(|v| ReturnSuccess::value(v.clone()))
                        .collect::<Vec<_>>(),
                ))
            } else {
                Ok(OutputStream::one(ReturnSuccess::value(v)))
            }
        }
        Err(e) => Err(e),
    }
}

pub fn calculate(values: &[Value], name: &Tag, mf: MathFunction) -> Result<Value, ShellError> {
    if values.iter().all(|v| v.is_primitive()) {
        mf(&values, &name)
    } else {
        // If we are not dealing with Primitives, then perhaps we are dealing with a table
        // Create a key for each column name
        let mut column_values = IndexMap::new();
        for value in values {
            if let UntaggedValue::Row(row_dict) = &value.value {
                for (key, value) in row_dict.entries.iter() {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert(vec![value.clone()]);
                }
            }
        }
        // The mathematical function operates over the columns of the table
        let mut column_totals = IndexMap::new();
        for (col_name, col_vals) in column_values {
            match mf(&col_vals, &name) {
                Ok(result) => {
                    column_totals.insert(col_name, result);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(UntaggedValue::Row(Dictionary {
            entries: column_totals,
        })
        .into_untagged_value())
    }
}
