use nu_engine::command_prelude::*;
use nu_protocol::Config;
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
};

#[derive(Clone)]
pub struct Join;

enum JoinType {
    Inner,
    Left,
    Right,
    Outer,
}

enum IncludeInner {
    No,
    Yes,
}

impl Command for Join {
    fn name(&self) -> &str {
        "join"
    }

    fn signature(&self) -> Signature {
        Signature::build("join")
            .required(
                "right-table",
                SyntaxShape::Table([].into()),
                "The right table in the join.",
            )
            .required(
                "left-on",
                SyntaxShape::String,
                "Name of column in input (left) table to join on.",
            )
            .optional(
                "right-on",
                SyntaxShape::String,
                "Name of column in right table to join on. Defaults to same column as left table.",
            )
            .switch("inner", "Inner join (default)", Some('i'))
            .switch("left", "Left-outer join", Some('l'))
            .switch("right", "Right-outer join", Some('r'))
            .switch("outer", "Outer join", Some('o'))
            .input_output_types(vec![(Type::table(), Type::table())])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Join two tables."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sql"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let metadata = input.metadata();
        let table_2: Value = call.req(engine_state, stack, 0)?;
        let l_on: Value = call.req(engine_state, stack, 1)?;
        let r_on: Value = call
            .opt(engine_state, stack, 2)?
            .unwrap_or_else(|| l_on.clone());
        let span = call.head;
        let join_type = join_type(engine_state, stack, call)?;

        // FIXME: we should handle ListStreams properly instead of collecting
        let collected_input = input.into_value(span)?;

        match (&collected_input, &table_2, &l_on, &r_on) {
            (
                Value::List { vals: rows_1, .. },
                Value::List { vals: rows_2, .. },
                Value::String { val: l_on, .. },
                Value::String { val: r_on, .. },
            ) => {
                let result = join(rows_1, rows_2, l_on, r_on, join_type, span);
                Ok(PipelineData::Value(result, metadata))
            }
            _ => Err(ShellError::UnsupportedInput {
                msg: "(PipelineData<table>, table, string, string)".into(),
                input: format!(
                    "({:?}, {:?}, {:?} {:?})",
                    collected_input,
                    table_2.get_type(),
                    l_on.get_type(),
                    r_on.get_type(),
                ),
                msg_span: span,
                input_span: span,
            }),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Join two tables",
            example: "[{a: 1 b: 2}] | join [{a: 1 c: 3}] a",
            result: Some(Value::test_list(vec![Value::test_record(record! {
                "a" => Value::test_int(1), "b" => Value::test_int(2), "c" => Value::test_int(3),
            })])),
        }]
    }
}

fn join_type(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<JoinType, nu_protocol::ShellError> {
    match (
        call.has_flag(engine_state, stack, "inner")?,
        call.has_flag(engine_state, stack, "left")?,
        call.has_flag(engine_state, stack, "right")?,
        call.has_flag(engine_state, stack, "outer")?,
    ) {
        (_, false, false, false) => Ok(JoinType::Inner),
        (false, true, false, false) => Ok(JoinType::Left),
        (false, false, true, false) => Ok(JoinType::Right),
        (false, false, false, true) => Ok(JoinType::Outer),
        _ => Err(ShellError::UnsupportedInput {
            msg: "Choose one of: --inner, --left, --right, --outer".into(),
            input: "".into(),
            msg_span: call.head,
            input_span: call.head,
        }),
    }
}

fn join(
    left: &[Value],
    right: &[Value],
    left_join_key: &str,
    right_join_key: &str,
    join_type: JoinType,
    span: Span,
) -> Value {
    // Inner / Right Join
    // ------------------
    // Make look-up table from rows on left
    // For each row r on right:
    //    If any matching rows on left:
    //        For each matching row l on left:
    //            Emit (l, r)
    //    Else if RightJoin:
    //        Emit (null, r)

    // Left Join
    // ----------
    // Make look-up table from rows on right
    // For each row l on left:
    //    If any matching rows on right:
    //        For each matching row r on right:
    //            Emit (l, r)
    //    Else:
    //        Emit (l, null)

    // Outer Join
    // ----------
    // Perform Left Join procedure
    // Perform Right Join procedure, but excluding rows in Inner Join

    let config = Config::default();
    let sep = ",";
    let cap = max(left.len(), right.len());
    let shared_join_key = if left_join_key == right_join_key {
        Some(left_join_key)
    } else {
        None
    };

    // For the "other" table, create a map from value in `on` column to a list of the
    // rows having that value.
    let mut result: Vec<Value> = Vec::new();
    let is_outer = matches!(join_type, JoinType::Outer);
    let (this, this_join_key, other, other_keys, join_type) = match join_type {
        JoinType::Left | JoinType::Outer => (
            left,
            left_join_key,
            lookup_table(right, right_join_key, sep, cap, &config),
            column_names(right),
            // For Outer we do a Left pass and a Right pass; this is the Left
            // pass.
            JoinType::Left,
        ),
        JoinType::Inner | JoinType::Right => (
            right,
            right_join_key,
            lookup_table(left, left_join_key, sep, cap, &config),
            column_names(left),
            join_type,
        ),
    };
    join_rows(
        &mut result,
        this,
        this_join_key,
        other,
        other_keys,
        shared_join_key,
        &join_type,
        IncludeInner::Yes,
        sep,
        &config,
        span,
    );
    if is_outer {
        let (this, this_join_key, other, other_names, join_type) = (
            right,
            right_join_key,
            lookup_table(left, left_join_key, sep, cap, &config),
            column_names(left),
            JoinType::Right,
        );
        join_rows(
            &mut result,
            this,
            this_join_key,
            other,
            other_names,
            shared_join_key,
            &join_type,
            IncludeInner::No,
            sep,
            &config,
            span,
        );
    }
    Value::list(result, span)
}

// Join rows of `this` (a nushell table) to rows of `other` (a lookup-table
// containing rows of a nushell table).
#[allow(clippy::too_many_arguments)]
fn join_rows(
    result: &mut Vec<Value>,
    this: &[Value],
    this_join_key: &str,
    other: HashMap<String, Vec<&Record>>,
    other_keys: Vec<&String>,
    shared_join_key: Option<&str>,
    join_type: &JoinType,
    include_inner: IncludeInner,
    sep: &str,
    config: &Config,
    span: Span,
) {
    if !this
        .iter()
        .any(|this_record| match this_record.as_record() {
            Ok(record) => record.contains(this_join_key),
            Err(_) => false,
        })
    {
        // `this` table does not contain the join column; do nothing
        return;
    }
    for this_row in this {
        if let Value::Record {
            val: this_record, ..
        } = this_row
        {
            if let Some(this_valkey) = this_record.get(this_join_key) {
                if let Some(other_rows) = other.get(&this_valkey.to_expanded_string(sep, config)) {
                    if matches!(include_inner, IncludeInner::Yes) {
                        for other_record in other_rows {
                            // `other` table contains rows matching `this` row on the join column
                            let record = match join_type {
                                JoinType::Inner | JoinType::Right => merge_records(
                                    other_record, // `other` (lookup) is the left input table
                                    this_record,
                                    shared_join_key,
                                ),
                                JoinType::Left => merge_records(
                                    this_record, // `this` is the left input table
                                    other_record,
                                    shared_join_key,
                                ),
                                _ => panic!("not implemented"),
                            };
                            result.push(Value::record(record, span))
                        }
                    }
                    continue;
                }
            }
            if !matches!(join_type, JoinType::Inner) {
                // Either `this` row is missing a value for the join column or
                // `other` table did not contain any rows matching
                // `this` row on the join column; emit a single joined
                // row with null values for columns not present
                let other_record = other_keys
                    .iter()
                    .map(|&key| {
                        let val = if Some(key.as_ref()) == shared_join_key {
                            this_record
                                .get(key)
                                .cloned()
                                .unwrap_or_else(|| Value::nothing(span))
                        } else {
                            Value::nothing(span)
                        };

                        (key.clone(), val)
                    })
                    .collect();

                let record = match join_type {
                    JoinType::Inner | JoinType::Right => {
                        merge_records(&other_record, this_record, shared_join_key)
                    }
                    JoinType::Left => merge_records(this_record, &other_record, shared_join_key),
                    _ => panic!("not implemented"),
                };

                result.push(Value::record(record, span))
            }
        };
    }
}

// Return column names (i.e. ordered keys from the first row; we assume that
// these are the same for all rows).
fn column_names(table: &[Value]) -> Vec<&String> {
    table
        .iter()
        .find_map(|val| match val {
            Value::Record { val, .. } => Some(val.columns().collect()),
            _ => None,
        })
        .unwrap_or_default()
}

// Create a map from value in `on` column to a list of the rows having that
// value.
fn lookup_table<'a>(
    rows: &'a [Value],
    on: &str,
    sep: &str,
    cap: usize,
    config: &Config,
) -> HashMap<String, Vec<&'a Record>> {
    let mut map = HashMap::<String, Vec<&'a Record>>::with_capacity(cap);
    for row in rows {
        if let Value::Record { val: record, .. } = row {
            if let Some(val) = record.get(on) {
                let valkey = val.to_expanded_string(sep, config);
                map.entry(valkey).or_default().push(record);
            }
        };
    }
    map
}

// Merge `left` and `right` records, renaming keys in `right` where they clash
// with keys in `left`. If `shared_key` is supplied then it is the name of a key
// that should not be renamed (its values are guaranteed to be equal).
fn merge_records(left: &Record, right: &Record, shared_key: Option<&str>) -> Record {
    let cap = max(left.len(), right.len());
    let mut seen = HashSet::with_capacity(cap);
    let mut record = Record::with_capacity(cap);
    for (k, v) in left {
        record.push(k.clone(), v.clone());
        seen.insert(k);
    }

    for (k, v) in right {
        let k_seen = seen.contains(k);
        let k_shared = shared_key == Some(k.as_str());
        // Do not output shared join key twice
        if !(k_seen && k_shared) {
            record.push(if k_seen { format!("{k}_") } else { k.clone() }, v.clone());
        }
    }
    record
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Join {})
    }
}
