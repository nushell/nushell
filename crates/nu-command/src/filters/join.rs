use nu_engine::command_prelude::*;
use nu_protocol::{Config, shell_error::generic::GenericError};
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
    Anti,
}

// The input table that `join_rows` iterates over (`this`). The other table is
// its lookup table. Merged rows always put the left table's columns first.
enum Side {
    Left,
    Right,
}

// Which rows of `this` table `join_rows` emits.
enum IncludeRows {
    // Rows with a match in the lookup table (inner join).
    Matched,
    // Rows without a match, padded with nulls (anti join, and the second pass
    // of an outer join).
    Unmatched,
    // Both (left and right joins, and the first pass of an outer join).
    All,
}

#[derive(Debug, Default)]
struct RightColumnRename {
    prefix: Option<String>,
    suffix: Option<String>,
}

impl Command for Join {
    fn name(&self) -> &str {
        "join"
    }

    fn signature(&self) -> Signature {
        Signature::build("join")
            .required(
                "right-table",
                SyntaxShape::table(),
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
            .named(
                "prefix",
                SyntaxShape::String,
                "Prefix columns from the right table with this string (excluding the shared join key).",
                Some('p'),
            )
            .named(
                "suffix",
                SyntaxShape::String,
                "Suffix columns from the right table with this string (excluding the shared join key).",
                Some('s'),
            )
            .switch("inner", "Inner join (default).", Some('i'))
            .switch("left", "Left-outer join.", Some('l'))
            .switch("right", "Right-outer join.", Some('r'))
            .switch("outer", "Outer join.", Some('o'))
            .switch("anti", "Anti join: keep rows with no match in right.", Some('a'))
            .input_output_types(vec![(Type::table(), Type::table())])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Join two tables."
    }

    fn extra_description(&self) -> &str {
        "If the input table or the right table has rows but none of them has its join column (`left-on` for the input, `right-on` for the right table), the join fails with a column-not-found error. This applies to every join type, including --left, --right, --outer and --anti. An empty table passes this check. In a table where only some rows have the join column, the other rows join as non-matching rows. To join a table whose rows all lack the column, add it first, for example with `default null <column>`."
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
        let mut input = input.into_stream_or_original(engine_state);

        let metadata = input.take_metadata();
        let table_2: Value = call.req(engine_state, stack, 0)?;
        let l_on: Value = call.req(engine_state, stack, 1)?;
        let r_on: Value = call
            .opt(engine_state, stack, 2)?
            .unwrap_or_else(|| l_on.clone());
        let span = call.head;
        let join_type = join_type(engine_state, stack, call)?;

        if matches!(join_type, JoinType::Anti) {
            let unsupported_anti_flags = ["prefix", "suffix"];
            for flag in unsupported_anti_flags {
                if call.has_flag(engine_state, stack, flag)? {
                    return Err(ShellError::UnsupportedInput {
                        msg: format!("--{flag} is a no-op under --anti"),
                        input: "".into(),
                        msg_span: call.head,
                        input_span: call.head,
                    });
                }
            }
        }

        let rename = RightColumnRename {
            prefix: call.get_flag(engine_state, stack, "prefix")?,
            suffix: call.get_flag(engine_state, stack, "suffix")?,
        };

        // FIXME: we should handle ListStreams properly instead of collecting
        let collected_input = input.into_value(span)?;

        match (&collected_input, &table_2, &l_on, &r_on) {
            (
                Value::List { vals: rows_1, .. },
                Value::List { vals: rows_2, .. },
                Value::String { val: l_key, .. },
                Value::String { val: r_key, .. },
            ) => {
                check_join_column(rows_1, l_key, l_on.span(), "input")?;
                check_join_column(rows_2, r_key, r_on.span(), "right")?;
                let result = join(rows_1, rows_2, l_key, r_key, join_type, &rename, span);
                Ok(PipelineData::value(result, metadata))
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Join two tables",
                example: "[{a: 1 b: 2}] | join [{a: 1 c: 3}] a",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(1), "b" => Value::test_int(2), "c" => Value::test_int(3),
                })])),
            },
            Example {
                description: "Join multiple tables with distinct suffixes for the right table's columns",
                example: "[{id: 1 x: 10}] | join --suffix _a [{id: 1 x: 20}] id | join --suffix _b [{id: 1 x: 30}] id",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "id" => Value::test_int(1),
                    "x" => Value::test_int(10),
                    "x_a" => Value::test_int(20),
                    "x_b" => Value::test_int(30),
                })])),
            },
            Example {
                description: "Join multiple tables with a prefix for the right table's columns",
                example: "[{id: 1 x: 10}] | join --prefix r_ [{id: 1 x: 20}] id",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "id" => Value::test_int(1),
                    "x" => Value::test_int(10),
                    "r_x" => Value::test_int(20),
                })])),
            },
            Example {
                description: "Pick a subset of rows without a matching row in the right table",
                example: "[{id: 1 x: 10}, {id: 2 x: 9}] | join --anti [{id: 1}] id",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "id" => Value::test_int(2),
                    "x" => Value::test_int(9),
                })])),
            },
            Example {
                description: "Left join a table whose rows all lack the join column by adding the column first",
                example: "[{id: 1 x: 10}] | join --left ([{y: 20}] | default null id) id",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "id" => Value::test_int(1),
                    "x" => Value::test_int(10),
                    "y" => Value::test_nothing(),
                })])),
            },
        ]
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
        call.has_flag(engine_state, stack, "anti")?,
    ) {
        (_, false, false, false, false) => Ok(JoinType::Inner),
        (false, true, false, false, false) => Ok(JoinType::Left),
        (false, false, true, false, false) => Ok(JoinType::Right),
        (false, false, false, true, false) => Ok(JoinType::Outer),
        (false, false, false, false, true) => Ok(JoinType::Anti),
        _ => Err(ShellError::UnsupportedInput {
            msg: "Choose one of: --inner, --left, --right, --outer, --anti".into(),
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
    rename: &RightColumnRename,
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
    //
    // Anti Join
    // ----------
    // Make look-up table from rows on right
    // For each row l on left:
    //      If not any matching rows on right:
    //          Emit (l)

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
    let include_rows = match join_type {
        JoinType::Inner => IncludeRows::Matched,
        JoinType::Left | JoinType::Right | JoinType::Outer => IncludeRows::All,
        JoinType::Anti => IncludeRows::Unmatched,
    };
    let (this, this_join_key, other, other_keys, this_side) = match join_type {
        JoinType::Left | JoinType::Outer => (
            left,
            left_join_key,
            lookup_table(right, right_join_key, sep, cap, &config),
            column_names(right),
            // For Outer we do a Left pass and a Right pass; this is the Left
            // pass.
            Side::Left,
        ),
        JoinType::Anti => (
            left,
            left_join_key,
            lookup_table(right, right_join_key, sep, cap, &config),
            // An anti join adds no columns from the right table, so the
            // unmatched left rows are emitted as they are.
            Vec::new(),
            Side::Left,
        ),
        JoinType::Inner | JoinType::Right => (
            right,
            right_join_key,
            lookup_table(left, left_join_key, sep, cap, &config),
            column_names(left),
            Side::Right,
        ),
    };
    join_rows(
        &mut result,
        this,
        this_join_key,
        other,
        other_keys,
        shared_join_key,
        this_side,
        include_rows,
        sep,
        &config,
        rename,
        span,
    );
    if is_outer {
        let (this, this_join_key, other, other_names) = (
            right,
            right_join_key,
            lookup_table(left, left_join_key, sep, cap, &config),
            column_names(left),
        );
        join_rows(
            &mut result,
            this,
            this_join_key,
            other,
            other_names,
            shared_join_key,
            Side::Right,
            IncludeRows::Unmatched,
            sep,
            &config,
            rename,
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
    this_side: Side,
    include_rows: IncludeRows,
    sep: &str,
    config: &Config,
    rename: &RightColumnRename,
    span: Span,
) {
    // Merge a row of `this` with a row of `other`, left input table first.
    let merge = |this_record: &Record, other_record: &Record| match this_side {
        Side::Left => merge_records(this_record, other_record, shared_join_key, rename),
        Side::Right => merge_records(other_record, this_record, shared_join_key, rename),
    };
    for this_row in this {
        if let Value::Record {
            val: this_record, ..
        } = this_row
        {
            if let Some(this_valkey) = this_record.get(this_join_key)
                && let Some(other_rows) = other.get(&this_valkey.to_expanded_string(sep, config))
            {
                if matches!(include_rows, IncludeRows::Matched | IncludeRows::All) {
                    for other_record in other_rows {
                        // `other` table contains rows matching `this` row on the join column
                        result.push(Value::record(merge(this_record, other_record), span))
                    }
                }
                continue;
            }
            if matches!(include_rows, IncludeRows::Unmatched | IncludeRows::All) {
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

                result.push(Value::record(merge(this_record, &other_record), span))
            }
        };
    }
}

// Fail when a non-empty table has no row with the join column, so a misspelled
// column name is reported instead of producing an empty or all-null join.
// Empty tables pass because they have no rows to show which columns they
// hold. Rows that lack the column in a table where other rows have it are
// still joined as non-matching rows. `table` names the table ("input" or
// "right") in the error, which points at the column argument. The error keeps
// the `column_not_found` code of `ShellError::CantFindColumn`, but not its help,
// which suggests `column?` optional access that `join` does not support.
fn check_join_column(
    rows: &[Value],
    column: &str,
    column_span: Span,
    table: &str,
) -> Result<(), ShellError> {
    let has_column = |row: &Value| row.as_record().is_ok_and(|record| record.contains(column));
    if rows.is_empty() || rows.iter().any(has_column) {
        Ok(())
    } else {
        Err(ShellError::Generic(
            GenericError::new(
                format!("Cannot find column '{column}'"),
                format!("no row in the {table} table has this column"),
                column_span,
            )
            .with_code("nu::shell::column_not_found")
            .with_help(
                "Check the column name. To join a table whose rows lack the column, add it first, for example with `default null <column>`",
            ),
        ))
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
        if let Value::Record { val: record, .. } = row
            && let Some(val) = record.get(on)
        {
            let valkey = val.to_expanded_string(sep, config);
            map.entry(valkey).or_default().push(record);
        };
    }
    map
}

// Merge `left` and `right` records, renaming keys in `right` where they clash
// with keys in `left`. If `shared_key` is supplied then it is the name of a key
// that should not be renamed (its values are guaranteed to be equal).
fn merge_records(
    left: &Record,
    right: &Record,
    shared_key: Option<&str>,
    rename: &RightColumnRename,
) -> Record {
    let cap = max(left.len(), right.len());
    let mut seen = HashSet::with_capacity(cap);
    let mut record = Record::with_capacity(cap);
    for (k, v) in left {
        record.push(k.clone(), v.clone());
        seen.insert(k.clone());
    }

    for (k, v) in right {
        let k_shared = shared_key == Some(k.as_str());
        // Do not output shared join key twice
        if k_shared && seen.contains(k) {
            continue;
        }

        let mut out_key = if rename.prefix.is_some() || rename.suffix.is_some() {
            format!(
                "{}{}{}",
                rename.prefix.as_deref().unwrap_or(""),
                k,
                rename.suffix.as_deref().unwrap_or("")
            )
        } else if seen.contains(k) {
            format!("{k}_")
        } else {
            k.clone()
        };

        // Ensure the output key is truly unique. If not, keep appending "_" until it is.
        while seen.contains(&out_key) {
            out_key.push('_');
        }

        record.push(out_key.clone(), v.clone());
        seen.insert(out_key);
    }
    record
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Join)
    }
}
