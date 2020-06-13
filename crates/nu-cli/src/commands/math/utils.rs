use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::hir::{convert_number_to_u64, Number, Operator};
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use num_traits::identities::Zero;

use indexmap::map::IndexMap;

pub type MathFunction = fn(values: &[Value], tag: &Tag) -> Result<Value, ShellError>;

pub async fn calculate(
    RunnableContext {
        mut input, name, ..
    }: RunnableContext,
    mf: MathFunction,
) -> Result<OutputStream, ShellError> {
    let values: Vec<Value> = input.drain_vec().await;

    if values.iter().all(|v| v.is_primitive()) {
        match mf(&values, &name) {
            Ok(result) => Ok(OutputStream::one(ReturnSuccess::value(result))),
            Err(err) => Err(err),
        }
    } else {
        let mut column_values = IndexMap::new();
        for value in values {
            if let UntaggedValue::Row(row_dict) = value.value {
                for (key, value) in row_dict.entries.iter() {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert(vec![value.clone()]);
                }
            }
        }

        let mut column_totals = IndexMap::new();
        for (col_name, col_vals) in column_values {
            match mf(&col_vals, &name) {
                Ok(result) => {
                    column_totals.insert(col_name, result);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(Dictionary {
                entries: column_totals,
            })
            .into_untagged_value(),
        )))
    }
}
