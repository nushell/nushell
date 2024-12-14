use nu_engine::command_prelude::*;

#[derive(Copy, Clone)]
pub(crate) enum MergeStrategy {
    /// Key-value pairs present in lhs and rhs are overwritten by values in rhs
    Shallow = 0,
    /// Tables are merged element-wise, other lists are overwritten by rhs
    Elementwise = 1,
    /// All lists are concatenated together
    Concatenation = 2,
}

fn is_table(val: &Value) -> bool {
    matches!(val.get_type(), Type::Table { .. })
}

pub(crate) fn do_merge(
    lhs: Value,
    rhs: Value,
    strategy: MergeStrategy,
    span: Span,
) -> Result<Value, ShellError> {
    match (strategy, lhs, rhs) {
        // Propagate errors
        (_, Value::Error { error, .. }, _) | (_, _, Value::Error { error, .. }) => Err(*error),
        // Deep merge records
        (
            MergeStrategy::Elementwise | MergeStrategy::Concatenation,
            Value::Record { val: lhs, .. },
            Value::Record { val: rhs, .. },
        ) => Ok(Value::record(
            merge_records(lhs.into_owned(), rhs.into_owned(), strategy, span)?,
            span,
        )),
        // Merge lists by concatenating
        (
            MergeStrategy::Concatenation,
            Value::List { vals: lhs, .. },
            Value::List { vals: rhs, .. },
        ) => Ok(Value::list(
            lhs.into_iter().chain(rhs.into_iter()).collect(),
            span,
        )),
        // Merge tables elementwise (but only if they are actually tables)
        (
            MergeStrategy::Elementwise,
            lhs_list @ Value::List { .. },
            rhs_list @ Value::List { .. },
        ) if is_table(&lhs_list) && is_table(&rhs_list) => {
            let lhs = lhs_list
                .into_list()
                .expect("Value matched as list above, but is not a list");
            let rhs = rhs_list
                .into_list()
                .expect("Value matched as list above, but is not a list");
            Ok(Value::list(merge_tables(lhs, rhs, span)?, span))
        }
        // Use rhs value (shallow record merge and general scalar merge)
        (_, _, val) => Ok(val),
    }
}

/// Merge right-hand table into left-hand table, element-wise
/// Assumes merge strategy is `MergeStrategy::Elementwise`
///
/// For example:
/// lhs = [{a: 12, b: 34}]
/// rhs = [{a: 56, c: 78}]
/// output = [{a: 56, b: 34, c: 78}]
fn merge_tables(lhs: Vec<Value>, rhs: Vec<Value>, span: Span) -> Result<Vec<Value>, ShellError> {
    let mut table_iter = rhs.into_iter();

    lhs.into_iter()
        .map(move |inp| match (inp.into_record(), table_iter.next()) {
            (Ok(rec), Some(to_merge)) => match to_merge.into_record() {
                Ok(to_merge) => Ok(Value::record(
                    merge_records(
                        rec.to_owned(),
                        to_merge.to_owned(),
                        MergeStrategy::Elementwise,
                        span,
                    )?,
                    span,
                )),
                Err(error) => Ok(Value::error(error, span)),
            },
            (Ok(rec), None) => Ok(Value::record(rec, span)),
            (Err(error), _) => Ok(Value::error(error, span)),
        })
        .collect()
}

fn merge_records(
    mut lhs: Record,
    rhs: Record,
    strategy: MergeStrategy,
    span: Span,
) -> Result<Record, ShellError> {
    for (col, rval) in rhs.into_iter() {
        // in order to both avoid cloning (possibly nested) record values and maintain the ordering of record keys, we can swap a temporary value into the source record.
        // if we were to remove the value, the ordering would be messed up as we might not insert back into the original index
        // it's okay to swap a temporary value in, since we know it will be replaced by the end of the function call
        //
        // use an error here instead of something like null so if this somehow makes it into the output, the bug will be immediately obvious
        let failed_error = ShellError::NushellFailed {
            msg: "Merge failed to properly replace internal temporary value".to_owned(),
        };

        let value = match lhs.insert(&col, Value::error(failed_error, span)) {
            Some(lval) => do_merge(lval, rval, strategy, span)?,
            None => rval,
        };

        lhs.insert(col, value);
    }
    Ok(lhs)
}
