use nu_engine::command_prelude::*;

#[derive(Copy, Clone)]
pub(crate) enum MergeStrategy {
    /// Key-value pairs present in lhs and rhs are overwritten by values in rhs
    Shallow,
    /// Records are merged recursively, otherwise same behavior as shallow
    Deep(ListMerge),
}

#[derive(Copy, Clone)]
pub(crate) enum ListMerge {
    /// Lists in lhs are overwritten by lists in rhs
    Overwrite,
    /// Lists of records are merged element-wise, other lists are overwritten by rhs
    Elementwise,
    /// All lists are concatenated together, lhs ++ rhs
    Append,
    /// All lists are concatenated together, rhs ++ lhs
    Prepend,
}

/// Test whether a value is a list of records.
///
/// This includes tables and non-tables.
fn is_list_of_records(val: &Value) -> bool {
    match val {
        list @ Value::List { .. } if matches!(list.get_type(), Type::Table { .. }) => true,
        // we want to include lists of records, but not lists of mixed types
        Value::List { vals, .. } => vals
            .iter()
            .map(Value::get_type)
            .all(|val| matches!(val, Type::Record { .. })),
        _ => false,
    }
}

/// Typecheck a merge operation.
///
/// Ensures that both arguments are records, tables, or lists of non-matching records.
pub(crate) fn typecheck_merge(lhs: &Value, rhs: &Value, head: Span) -> Result<(), ShellError> {
    match (lhs.get_type(), rhs.get_type()) {
        (Type::Record { .. }, Type::Record { .. }) => Ok(()),
        (_, _) if is_list_of_records(lhs) && is_list_of_records(rhs) => Ok(()),
        _ => Err(ShellError::PipelineMismatch {
            exp_input_type: "input and argument to be both record or both table".to_string(),
            dst_span: head,
            src_span: lhs.span(),
        }),
    }
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
        // Shallow merge records
        (
            MergeStrategy::Shallow,
            Value::Record { val: lhs, .. },
            Value::Record { val: rhs, .. },
        ) => Ok(Value::record(
            merge_records(lhs.into_owned(), rhs.into_owned(), strategy, span)?,
            span,
        )),
        // Deep merge records
        (
            MergeStrategy::Deep(_),
            Value::Record { val: lhs, .. },
            Value::Record { val: rhs, .. },
        ) => Ok(Value::record(
            merge_records(lhs.into_owned(), rhs.into_owned(), strategy, span)?,
            span,
        )),
        // Merge lists by appending
        (
            MergeStrategy::Deep(ListMerge::Append),
            Value::List { vals: lhs, .. },
            Value::List { vals: rhs, .. },
        ) => Ok(Value::list(lhs.into_iter().chain(rhs).collect(), span)),
        // Merge lists by prepending
        (
            MergeStrategy::Deep(ListMerge::Prepend),
            Value::List { vals: lhs, .. },
            Value::List { vals: rhs, .. },
        ) => Ok(Value::list(rhs.into_iter().chain(lhs).collect(), span)),
        // Merge lists of records elementwise (tables and non-tables)
        // Match on shallow since this might be a top-level table
        (
            MergeStrategy::Shallow | MergeStrategy::Deep(ListMerge::Elementwise),
            lhs_list @ Value::List { .. },
            rhs_list @ Value::List { .. },
        ) if is_list_of_records(&lhs_list) && is_list_of_records(&rhs_list) => {
            let lhs = lhs_list
                .into_list()
                .expect("Value matched as list above, but is not a list");
            let rhs = rhs_list
                .into_list()
                .expect("Value matched as list above, but is not a list");
            Ok(Value::list(merge_tables(lhs, rhs, strategy, span)?, span))
        }
        // Use rhs value (shallow record merge, overwrite list merge, and general scalar merge)
        (_, _, val) => Ok(val),
    }
}

/// Merge right-hand table into left-hand table, element-wise
///
/// For example:
/// lhs = [{a: 12, b: 34}]
/// rhs = [{a: 56, c: 78}]
/// output = [{a: 56, b: 34, c: 78}]
fn merge_tables(
    lhs: Vec<Value>,
    rhs: Vec<Value>,
    strategy: MergeStrategy,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut table_iter = rhs.into_iter();

    lhs.into_iter()
        .map(move |inp| match (inp.into_record(), table_iter.next()) {
            (Ok(rec), Some(to_merge)) => match to_merge.into_record() {
                Ok(to_merge) => Ok(Value::record(
                    merge_records(rec.to_owned(), to_merge.to_owned(), strategy, span)?,
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
    match strategy {
        MergeStrategy::Shallow => {
            for (col, rval) in rhs.into_iter() {
                lhs.insert(col, rval);
            }
        }
        strategy => {
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
        }
    }
    Ok(lhs)
}
