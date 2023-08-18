use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::cmp::max;
use std::collections::{HashMap, HashSet};

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

type RowEntries<'a> = Vec<(&'a Vec<String>, &'a Vec<Value>)>;

const EMPTY_COL_NAMES: &Vec<String> = &vec![];

impl Command for Join {
    fn name(&self) -> &str {
        "join"
    }

    fn signature(&self) -> Signature {
        Signature::build("join")
            .required(
                "right-table",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "The right table in the join",
            )
            .required(
                "left-on",
                SyntaxShape::String,
                "Name of column in input (left) table to join on",
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
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Join two tables"
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
        let table_2: Value = call.req(engine_state, stack, 0)?;
        let l_on: Value = call.req(engine_state, stack, 1)?;
        let r_on: Value = call
            .opt(engine_state, stack, 2)?
            .unwrap_or_else(|| l_on.clone());
        let span = call.head;
        let join_type = join_type(call)?;

        // FIXME: we should handle ListStreams properly instead of collecting
        let collected_input = input.into_value(span);

        match (&collected_input, &table_2, &l_on, &r_on) {
            (
                Value::List { vals: rows_1, .. },
                Value::List { vals: rows_2, .. },
                Value::String { val: l_on, .. },
                Value::String { val: r_on, .. },
            ) => {
                let result = join(rows_1, rows_2, l_on, r_on, join_type, span);
                Ok(PipelineData::Value(result, None))
            }
            _ => Err(ShellError::UnsupportedInput(
                "(PipelineData<table>, table, string, string)".into(),
                format!(
                    "({:?}, {:?}, {:?} {:?})",
                    collected_input,
                    table_2.get_type(),
                    l_on.get_type(),
                    r_on.get_type(),
                ),
                span,
                span,
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Join two tables",
            example: "[{a: 1 b: 2}] | join [{a: 1 c: 3}] a",
            result: Some(Value::List {
                vals: vec![Value::Record {
                    cols: vec!["a".into(), "b".into(), "c".into()],
                    vals: vec![
                        Value::Int {
                            val: 1,
                            span: Span::test_data(),
                        },
                        Value::Int {
                            val: 2,
                            span: Span::test_data(),
                        },
                        Value::Int {
                            val: 3,
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }],
                span: Span::test_data(),
            }),
        }]
    }
}

fn join_type(call: &Call) -> Result<JoinType, nu_protocol::ShellError> {
    match (
        call.has_flag("inner"),
        call.has_flag("left"),
        call.has_flag("right"),
        call.has_flag("outer"),
    ) {
        (_, false, false, false) => Ok(JoinType::Inner),
        (false, true, false, false) => Ok(JoinType::Left),
        (false, false, true, false) => Ok(JoinType::Right),
        (false, false, false, true) => Ok(JoinType::Outer),
        _ => Err(ShellError::UnsupportedInput(
            "Choose one of: --inner, --left, --right, --outer".into(),
            "".into(),
            call.head,
            call.head,
        )),
    }
}

fn join(
    left: &Vec<Value>,
    right: &Vec<Value>,
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
    Value::List { vals: result, span }
}

// Join rows of `this` (a nushell table) to rows of `other` (a lookup-table
// containing rows of a nushell table).
#[allow(clippy::too_many_arguments)]
fn join_rows(
    result: &mut Vec<Value>,
    this: &Vec<Value>,
    this_join_key: &str,
    other: HashMap<String, RowEntries>,
    other_keys: &Vec<String>,
    shared_join_key: Option<&str>,
    join_type: &JoinType,
    include_inner: IncludeInner,
    sep: &str,
    config: &Config,
    span: Span,
) {
    for this_row in this {
        if let Value::Record {
            cols: this_cols,
            vals: this_vals,
            ..
        } = this_row
        {
            if let Some(this_valkey) = this_row.get_data_by_key(this_join_key) {
                if let Some(other_rows) = other.get(&this_valkey.into_string(sep, config)) {
                    if matches!(include_inner, IncludeInner::Yes) {
                        for (other_cols, other_vals) in other_rows {
                            // `other` table contains rows matching `this` row on the join column
                            let (res_cols, res_vals) = match join_type {
                                JoinType::Inner | JoinType::Right => merge_records(
                                    (other_cols, other_vals), // `other` (lookup) is the left input table
                                    (this_cols, this_vals),
                                    shared_join_key,
                                ),
                                JoinType::Left => merge_records(
                                    (this_cols, this_vals), // `this` is the left input table
                                    (other_cols, other_vals),
                                    shared_join_key,
                                ),
                                _ => panic!("not implemented"),
                            };
                            result.push(Value::Record {
                                cols: res_cols,
                                vals: res_vals,
                                span,
                            })
                        }
                    }
                } else if !matches!(join_type, JoinType::Inner) {
                    // `other` table did not contain any rows matching
                    // `this` row on the join column; emit a single joined
                    // row with null values for columns not present,
                    let other_vals = other_keys
                        .iter()
                        .map(|key| {
                            if Some(key.as_ref()) == shared_join_key {
                                this_row
                                    .get_data_by_key(key)
                                    .unwrap_or_else(|| Value::nothing(span))
                            } else {
                                Value::nothing(span)
                            }
                        })
                        .collect();
                    let (res_cols, res_vals) = match join_type {
                        JoinType::Inner | JoinType::Right => merge_records(
                            (other_keys, &other_vals),
                            (this_cols, this_vals),
                            shared_join_key,
                        ),
                        JoinType::Left => merge_records(
                            (this_cols, this_vals),
                            (other_keys, &other_vals),
                            shared_join_key,
                        ),
                        _ => panic!("not implemented"),
                    };

                    result.push(Value::Record {
                        cols: res_cols,
                        vals: res_vals,
                        span,
                    })
                }
            } // else { a row is missing a value for the join column }
        };
    }
}

// Return column names (i.e. ordered keys from the first row; we assume that
// these are the same for all rows).
fn column_names(table: &[Value]) -> &Vec<String> {
    table
        .iter()
        .find_map(|val| match val {
            Value::Record { cols, .. } => Some(cols),
            _ => None,
        })
        .unwrap_or(EMPTY_COL_NAMES)
}

// Create a map from value in `on` column to a list of the rows having that
// value.
fn lookup_table<'a>(
    rows: &'a Vec<Value>,
    on: &str,
    sep: &str,
    cap: usize,
    config: &Config,
) -> HashMap<String, RowEntries<'a>> {
    let mut map = HashMap::<String, RowEntries>::with_capacity(cap);
    for row in rows {
        if let Value::Record { cols, vals, .. } = row {
            if let Some(val) = &row.get_data_by_key(on) {
                let valkey = val.into_string(sep, config);
                map.entry(valkey).or_default().push((cols, vals));
            }
        };
    }
    map
}

// Merge `left` and `right` records, renaming keys in `right` where they clash
// with keys in `left`. If `shared_key` is supplied then it is the name of a key
// that should not be renamed (its values are guaranteed to be equal).
fn merge_records(
    left: (&Vec<String>, &Vec<Value>),
    right: (&Vec<String>, &Vec<Value>),
    shared_key: Option<&str>,
) -> (Vec<String>, Vec<Value>) {
    let ((l_keys, l_vals), (r_keys, r_vals)) = (left, right);
    let cap = max(l_keys.len(), r_keys.len());
    let mut seen = HashSet::with_capacity(cap);
    let (mut res_keys, mut res_vals) = (Vec::with_capacity(cap), Vec::with_capacity(cap));
    for (k, v) in l_keys.iter().zip(l_vals) {
        res_keys.push(k.clone());
        res_vals.push(v.clone());
        seen.insert(k);
    }

    for (k, v) in r_keys.iter().zip(r_vals) {
        let k_seen = seen.contains(k);
        let k_shared = shared_key == Some(k);
        // Do not output shared join key twice
        if !(k_seen && k_shared) {
            res_keys.push(if k_seen { format!("{}_", k) } else { k.clone() });
            res_vals.push(v.clone());
        }
    }
    (res_keys, res_vals)
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
