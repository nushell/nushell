use indexmap::IndexMap;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{
    FromValue, Range, ast::PathMember, engine::Closure, shell_error::generic::GenericError,
};
use std::{
    hash::{Hash, Hasher},
    ops::Bound,
};

#[derive(Clone)]
pub struct GroupBy;

impl Command for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by")
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Any)])
            .switch(
                "to-table",
                "Return a table with \"groups\" and \"items\" columns.",
                None,
            )
            .switch(
                "prune",
                "Remove a column after grouping, if applicable.",
                None,
            )
            .rest(
                "grouper",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The path to the column to group on.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Splits a list or table into groups. Returns a record when the group keys are strings, otherwise a table."
    }

    fn extra_description(&self) -> &str {
        r#"Grouping uses typed value identity (same type and payload), not display strings. Distinct filesizes that render the same (for example 1MB and 1.001MB) stay in separate groups. "true" and true are separate groups.

The default output is a record when every group key is a string. Those record keys are the original strings; they are not reformatted. null group keys are omitted (records cannot use null as a key). Optional cell paths (e.g. `foo?`) still ignore rows where access yields null.

If any group key is not a string, the default output is a table with the original key types — the same shape as --to-table.

--to-table always returns a table. Group columns keep their original types. null group keys are included as null values."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        group_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Group items by the \"type\" column's values.",
                example: "ls | group-by type",
                result: None,
            },
            Example {
                description: "Group items by the \"foo\" column's values, ignoring records without a \"foo\" column.",
                example: "open cool.json | group-by foo?",
                result: None,
            },
            Example {
                description: "Group using a block which is evaluated against each input value.",
                example: "[foo.txt bar.csv baz.txt] | group-by { path parse | get extension }",
                result: Some(Value::test_record(record! {
                    "txt" => Value::test_list(vec![
                        Value::test_string("foo.txt"),
                        Value::test_string("baz.txt"),
                    ]),
                    "csv" => Value::test_list(vec![Value::test_string("bar.csv")]),
                })),
            },
            Example {
                description: "You can also group by raw values by leaving out the argument.",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by",
                result: Some(Value::test_record(record! {
                    "1" => Value::test_list(vec![
                        Value::test_string("1"),
                        Value::test_string("1"),
                        Value::test_string("1"),
                        Value::test_string("1"),
                    ]),
                    "3" => Value::test_list(vec![
                        Value::test_string("3"),
                        Value::test_string("3"),
                    ]),
                    "2" => Value::test_list(vec![Value::test_string("2")]),
                })),
            },
            Example {
                description: "Group by a non-string column. The result is a table so the keys keep their original type.",
                example: "[{n: 1} {n: 2} {n: 1}] | group-by n",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "n" => Value::test_int(1),
                        "items" => Value::test_list(vec![
                            Value::test_record(record! { "n" => Value::test_int(1) }),
                            Value::test_record(record! { "n" => Value::test_int(1) }),
                        ]),
                    }),
                    Value::test_record(record! {
                        "n" => Value::test_int(2),
                        "items" => Value::test_list(vec![
                            Value::test_record(record! { "n" => Value::test_int(2) }),
                        ]),
                    }),
                ])),
            },
            Example {
                description: "You can also output a table instead of a record.",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by --to-table",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "group" => Value::test_string("1"),
                        "items" => Value::test_list(vec![
                            Value::test_string("1"),
                            Value::test_string("1"),
                            Value::test_string("1"),
                            Value::test_string("1"),
                        ]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("3"),
                        "items" => Value::test_list(vec![
                            Value::test_string("3"),
                            Value::test_string("3"),
                        ]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("2"),
                        "items" => Value::test_list(vec![Value::test_string("2")]),
                    }),
                ])),
            },
            Example {
                description: "Bools and strings are different keys, so the result is a table.",
                example: r#"[true "true" false "false"] | group-by"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "group" => Value::test_bool(true),
                        "items" => Value::test_list(vec![Value::test_bool(true)]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("true"),
                        "items" => Value::test_list(vec![Value::test_string("true")]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_bool(false),
                        "items" => Value::test_list(vec![Value::test_bool(false)]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("false"),
                        "items" => Value::test_list(vec![Value::test_string("false")]),
                    }),
                ])),
            },
            Example {
                description: "Group items by multiple columns' values.",
                example: r#"[
        [name, lang, year];
        [andres, rb, "2019"],
        [jt, rs, "2019"],
        [storm, rs, "2021"]
    ]
    | group-by lang year"#,
                result: Some(Value::test_record(record! {
                    "rb" => Value::test_record(record! {
                        "2019" => Value::test_list(
                            vec![Value::test_record(record! {
                                    "name" => Value::test_string("andres"),
                                    "lang" => Value::test_string("rb"),
                                    "year" => Value::test_string("2019"),
                            })],
                        ),
                    }),
                    "rs" => Value::test_record(record! {
                            "2019" => Value::test_list(
                                vec![Value::test_record(record! {
                                        "name" => Value::test_string("jt"),
                                        "lang" => Value::test_string("rs"),
                                        "year" => Value::test_string("2019"),
                                })],
                            ),
                            "2021" => Value::test_list(
                                vec![Value::test_record(record! {
                                        "name" => Value::test_string("storm"),
                                        "lang" => Value::test_string("rs"),
                                        "year" => Value::test_string("2021"),
                                })],
                            ),
                    }),
                })),
            },
            Example {
                description: "Group items by multiple columns' values.",
                example: r#"[
        [name, lang, year];
        [andres, rb, "2019"],
        [jt, rs, "2019"],
        [storm, rs, "2021"]
    ]
    | group-by lang year --to-table"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "lang" => Value::test_string("rb"),
                        "year" => Value::test_string("2019"),
                        "items" => Value::test_list(vec![
                            Value::test_record(record! {
                                "name" => Value::test_string("andres"),
                                "lang" => Value::test_string("rb"),
                                "year" => Value::test_string("2019"),
                            })
                        ]),
                    }),
                    Value::test_record(record! {
                        "lang" => Value::test_string("rs"),
                        "year" => Value::test_string("2019"),
                        "items" => Value::test_list(vec![
                            Value::test_record(record! {
                                "name" => Value::test_string("jt"),
                                "lang" => Value::test_string("rs"),
                                "year" => Value::test_string("2019"),
                            })
                        ]),
                    }),
                    Value::test_record(record! {
                        "lang" => Value::test_string("rs"),
                        "year" => Value::test_string("2021"),
                        "items" => Value::test_list(vec![
                            Value::test_record(record! {
                                "name" => Value::test_string("storm"),
                                "lang" => Value::test_string("rs"),
                                "year" => Value::test_string("2021"),
                            })
                        ]),
                    }),
                ])),
            },
            Example {
                description: "Group items by column and delete the original.",
                example: r#"[
        [name, lang, year];
        [andres, rb, "2019"],
        [jt, rs, "2019"],
        [storm, rs, "2021"]
    ]
    | group-by lang --prune"#,
                #[cfg(test)] // Cannot test this example, it requires the nu-cmd-extra crate.
                result: None,
                #[cfg(not(test))]
                result: Some(Value::test_record(record! {
                        "rb" => Value::test_list(vec![Value::test_record(record! {
                                        "name" => Value::test_string("andres"),
                                        "year" => Value::test_string("2019"),
                                })],
                            ),
                        "rs" => Value::test_list(
                                    vec![
                                    Value::test_record(record! {
                                            "name" => Value::test_string("jt"),
                                            "year" => Value::test_string("2019"),
                                    }),
                                    Value::test_record(record! {
                                            "name" => Value::test_string("storm"),
                                            "year" => Value::test_string("2021"),
                                    })
                            ]),
                })),
            },
        ]
    }
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let groupers: Vec<Spanned<Grouper>> = call.rest(engine_state, stack, 0)?;
    let to_table = call.has_flag(engine_state, stack, "to-table")?;
    let prune = call.has_flag(engine_state, stack, "prune")?;

    let values: Vec<Value> = input.into_iter().collect();
    if values.is_empty() {
        let val = if to_table {
            Value::list(Vec::new(), head)
        } else {
            Value::record(Record::new(), head)
        };
        return Ok(val.into_pipeline_data());
    }

    let grouped = match &groupers[..] {
        [first, rest @ ..] => {
            let mut grouped = Grouped::new(first.as_ref(), prune, values, engine_state, stack)?;
            for grouper in rest {
                grouped.subgroup(grouper.as_ref(), prune, engine_state, stack)?;
            }
            grouped
        }
        [] => Grouped::empty(values),
    };

    // Records can only use string keys. Non-string group keys keep their type
    // by emitting the same table --to-table would.
    let value = if to_table || grouped.has_non_string_key() {
        let column_names = groupers_to_column_names(&groupers)?;
        grouped.into_table(&column_names, head)
    } else {
        grouped.into_record(head)
    };

    Ok(value.into_pipeline_data())
}

fn groupers_to_column_names(groupers: &[Spanned<Grouper>]) -> Result<Vec<String>, ShellError> {
    if groupers.is_empty() {
        return Ok(vec!["group".into(), "items".into()]);
    }

    let mut closure_idx: usize = 0;
    let grouper_names = groupers.iter().map(|grouper| {
        grouper.as_ref().map(|item| match item {
            Grouper::CellPath { val } => val.to_column_name(),
            Grouper::Closure { .. } => {
                closure_idx += 1;
                format!("closure_{}", closure_idx - 1)
            }
        })
    });

    let mut name_set: Vec<Spanned<String>> = Vec::with_capacity(grouper_names.len());

    for name in grouper_names {
        if name.item == "items" {
            return Err(ShellError::Generic(
                GenericError::new(
                    "grouper arguments can't be named `items`",
                    "here",
                    name.span,
                )
                .with_help("instead of a cell-path, try using a closure: { get items }"),
            ));
        }

        if let Some(conflicting_name) = name_set
            .iter()
            .find(|elem| elem.as_ref().item == name.item.as_str())
        {
            return Err(ShellError::Generic(
                GenericError::new(
                    "grouper arguments result in colliding column names",
                    "duplicate column names",
                    conflicting_name.span.append(name.span),
                )
                .with_help("instead of a cell-path, try using a closure or renaming columns")
                .with_inner([ShellError::ColumnDefinedTwice {
                    col_name: conflicting_name.item.clone(),
                    first_use: conflicting_name.span,
                    second_use: name.span,
                }]),
            ));
        }

        name_set.push(name);
    }

    let column_names: Vec<String> = name_set
        .into_iter()
        .map(|elem| elem.item)
        .chain(["items".into()])
        .collect();
    Ok(column_names)
}

/// Internal group key. `Nothing` is distinct from the empty string so null and `""`
/// do not collapse. Record output omits `Nothing` keys; table output emits them as null.
#[derive(Debug, Clone)]
enum GroupKey {
    Nothing,
    Preserved(Value),
}

impl GroupKey {
    fn from_value(value: &Value) -> Self {
        if value.is_nothing() {
            Self::Nothing
        } else {
            Self::Preserved(value.clone())
        }
    }

    fn into_value(self, span: Span) -> Value {
        match self {
            Self::Nothing => Value::nothing(span),
            Self::Preserved(v) => v,
        }
    }

    fn as_record_key(&self) -> Option<&str> {
        match self {
            Self::Preserved(Value::String { val, .. }) => Some(val.as_str()),
            Self::Nothing | Self::Preserved(_) => None,
        }
    }

    fn is_non_string(&self) -> bool {
        !matches!(self, Self::Nothing | Self::Preserved(Value::String { .. }))
    }
}

impl PartialEq for GroupKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nothing, Self::Nothing) => true,
            (Self::Preserved(a), Self::Preserved(b)) => group_values_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for GroupKey {}

impl Hash for GroupKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Nothing => {}
            Self::Preserved(v) => hash_group_value(v, state),
        }
    }
}

fn hash_group_value<H: Hasher>(value: &Value, state: &mut H) {
    std::mem::discriminant(value).hash(state);
    match value {
        Value::Bool { val, .. } => val.hash(state),
        Value::Int { val, .. } => val.hash(state),
        Value::Float { val, .. } => val.to_bits().hash(state),
        Value::String { val, .. } => val.hash(state),
        Value::Glob { val, no_expand, .. } => {
            val.hash(state);
            no_expand.hash(state);
        }
        Value::Filesize { val, .. } => val.hash(state),
        Value::Duration { val, .. } => val.hash(state),
        Value::Date { val, .. } => val.hash(state),
        Value::Range { val, .. } => hash_range(val, state),
        Value::Record { val, .. } => {
            let mut pairs: Vec<_> = val.iter().collect();
            pairs.sort_unstable_by_key(|(a, _)| *a);
            pairs.len().hash(state);
            for (key, child) in pairs {
                key.hash(state);
                hash_group_value(child, state);
            }
        }
        Value::List { vals, .. } => {
            vals.len().hash(state);
            for child in vals.iter() {
                hash_group_value(child, state);
            }
        }
        Value::Closure { val, .. } => val.block_id.hash(state),
        Value::Error { .. } => {}
        Value::Binary { val, .. } => val.hash(state),
        Value::CellPath { val, .. } => {
            val.members.len().hash(state);
            for member in &val.members {
                match member {
                    PathMember::String { val, optional, .. } => {
                        0u8.hash(state);
                        val.hash(state);
                        optional.hash(state);
                    }
                    PathMember::Int { val, optional, .. } => {
                        1u8.hash(state);
                        val.hash(state);
                        optional.hash(state);
                    }
                }
            }
        }
        Value::Custom { val, .. } => {
            val.type_name().hash(state);
            if let Ok(base) = val.to_base_value(value.span()) {
                hash_group_value(&base, state);
            }
        }
        Value::Nothing { .. } => {}
    }
}

fn hash_range<H: Hasher>(range: &Range, state: &mut H) {
    match range {
        Range::IntRange(r) => {
            0u8.hash(state);
            r.start().hash(state);
            r.step().hash(state);
            r.end().hash(state);
        }
        Range::FloatRange(r) => {
            1u8.hash(state);
            r.start().to_bits().hash(state);
            r.step().to_bits().hash(state);
            match r.end() {
                Bound::Unbounded => 0u8.hash(state),
                Bound::Included(v) => {
                    1u8.hash(state);
                    v.to_bits().hash(state);
                }
                Bound::Excluded(v) => {
                    2u8.hash(state);
                    v.to_bits().hash(state);
                }
            }
        }
    }
}

fn ranges_eq(left: &Range, right: &Range) -> bool {
    match (left, right) {
        (Range::IntRange(a), Range::IntRange(b)) => a == b,
        (Range::FloatRange(a), Range::FloatRange(b)) => {
            a.start().to_bits() == b.start().to_bits()
                && a.step().to_bits() == b.step().to_bits()
                && match (a.end(), b.end()) {
                    (Bound::Unbounded, Bound::Unbounded) => true,
                    (Bound::Included(x), Bound::Included(y))
                    | (Bound::Excluded(x), Bound::Excluded(y)) => x.to_bits() == y.to_bits(),
                    _ => false,
                }
        }
        _ => false,
    }
}

fn group_values_eq(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Bool { val: a, .. }, Value::Bool { val: b, .. }) => a == b,
        (Value::Int { val: a, .. }, Value::Int { val: b, .. }) => a == b,
        (Value::Float { val: a, .. }, Value::Float { val: b, .. }) => a.to_bits() == b.to_bits(),
        (Value::String { val: a, .. }, Value::String { val: b, .. }) => a == b,
        (
            Value::Glob {
                val: a,
                no_expand: a_no_expand,
                ..
            },
            Value::Glob {
                val: b,
                no_expand: b_no_expand,
                ..
            },
        ) => a == b && a_no_expand == b_no_expand,
        (Value::Filesize { val: a, .. }, Value::Filesize { val: b, .. }) => a == b,
        (Value::Duration { val: a, .. }, Value::Duration { val: b, .. }) => a == b,
        (Value::Date { val: a, .. }, Value::Date { val: b, .. }) => a == b,
        (Value::Range { val: a, .. }, Value::Range { val: b, .. }) => ranges_eq(a, b),
        (Value::Record { val: a, .. }, Value::Record { val: b, .. }) => {
            if a.len() != b.len() {
                return false;
            }
            let mut left_pairs: Vec<_> = a.iter().collect();
            let mut right_pairs: Vec<_> = b.iter().collect();
            left_pairs.sort_unstable_by_key(|(x, _)| *x);
            right_pairs.sort_unstable_by_key(|(x, _)| *x);
            left_pairs
                .iter()
                .zip(right_pairs)
                .all(|((ak, av), (bk, bv))| *ak == bk && group_values_eq(av, bv))
        }
        (Value::List { vals: a, .. }, Value::List { vals: b, .. }) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| group_values_eq(x, y))
        }
        (Value::Closure { val: a, .. }, Value::Closure { val: b, .. }) => a.block_id == b.block_id,
        (Value::Error { .. }, Value::Error { .. }) => true,
        (Value::Binary { val: a, .. }, Value::Binary { val: b, .. }) => a == b,
        (Value::CellPath { val: a, .. }, Value::CellPath { val: b, .. }) => a == b,
        (Value::Custom { val: a, .. }, Value::Custom { val: b, .. }) => {
            if a.type_name() != b.type_name() {
                return false;
            }
            match (a.to_base_value(left.span()), b.to_base_value(right.span())) {
                (Ok(a_base), Ok(b_base)) => group_values_eq(&a_base, &b_base),
                (Err(_), Err(_)) => true,
                _ => false,
            }
        }
        (Value::Nothing { .. }, Value::Nothing { .. }) => true,
        _ => false,
    }
}

fn path_has_optional_member(column_name: &CellPath) -> bool {
    column_name.members.iter().any(|member| match member {
        PathMember::String { optional, .. } => *optional,
        PathMember::Int { optional, .. } => *optional,
    })
}

fn group_cell_path(
    column_name: &CellPath,
    prune: bool,
    values: Vec<Value>,
) -> Result<IndexMap<GroupKey, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let optional_path = path_has_optional_member(column_name);

    for mut value in values.into_iter() {
        let key_val = value.follow_cell_path(&column_name.members)?;

        // Optional cell paths (`col?`) drop rows when access yields nothing (missing
        // column or explicit null). Required paths keep null as a distinct group key.
        if key_val.is_nothing() && optional_path {
            continue;
        }

        let key = GroupKey::from_value(key_val.as_ref());

        if prune {
            // it's okay if this fails since pruning is best-effort
            let _ = value.remove_data_at_cell_path(&column_name.members);

            // also try pruning parent, if it has now become empty
            let parent = column_name.members.split_last().map(|(_, head)| head);

            if let Some(parent) = parent
                && let Ok(parent_value) = value.follow_cell_path(parent)
                && parent_value.is_empty()
            {
                let _ = value.remove_data_at_cell_path(parent);
            }
        }

        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn group_closure(
    values: Vec<Value>,
    span: Span,
    closure: Closure,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<IndexMap<GroupKey, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let mut closure = ClosureEval::new(engine_state, stack, closure);

    for value in values {
        let key_val = closure.run_with_value(value.clone())?.into_value(span)?;
        let key = GroupKey::from_value(&key_val);

        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

enum Grouper {
    CellPath { val: CellPath },
    Closure { val: Box<Closure> },
}

impl FromValue for Grouper {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::CellPath { val, .. } => Ok(Grouper::CellPath { val }),
            Value::Closure { val, .. } => Ok(Grouper::Closure { val }),
            _ => Err(ShellError::TypeMismatch {
                err_message: "unsupported grouper type".to_string(),
                span: v.span(),
            }),
        }
    }
}

struct Grouped {
    groups: Tree,
}

enum Tree {
    Leaf(IndexMap<GroupKey, Vec<Value>>),
    Branch(IndexMap<GroupKey, Grouped>),
}

impl Grouped {
    fn empty(values: Vec<Value>) -> Self {
        let mut groups = IndexMap::<_, Vec<_>>::new();

        for value in values.into_iter() {
            let key = GroupKey::from_value(&value);
            groups.entry(key).or_default().push(value);
        }

        Self {
            groups: Tree::Leaf(groups),
        }
    }

    fn new(
        grouper: Spanned<&Grouper>,
        prune: bool,
        values: Vec<Value>,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Self, ShellError> {
        let groups = match grouper.item {
            Grouper::CellPath { val } => group_cell_path(val, prune, values)?,
            Grouper::Closure { val } => group_closure(
                values,
                grouper.span,
                Closure::clone(val),
                engine_state,
                stack,
            )?,
        };
        Ok(Self {
            groups: Tree::Leaf(groups),
        })
    }

    fn subgroup(
        &mut self,
        grouper: Spanned<&Grouper>,
        prune: bool,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<(), ShellError> {
        let groups = match &mut self.groups {
            Tree::Leaf(groups) => std::mem::take(groups)
                .into_iter()
                .map(|(key, values)| -> Result<_, ShellError> {
                    let leaf = Self::new(grouper, prune, values, engine_state, stack)?;
                    Ok((key, leaf))
                })
                .collect::<Result<IndexMap<_, _>, ShellError>>()?,
            Tree::Branch(nested_groups) => {
                let mut nested_groups = std::mem::take(nested_groups);
                for v in nested_groups.values_mut() {
                    v.subgroup(grouper, prune, engine_state, stack)?;
                }
                nested_groups
            }
        };
        self.groups = Tree::Branch(groups);
        Ok(())
    }

    fn has_non_string_key(&self) -> bool {
        match &self.groups {
            Tree::Leaf(leaf) => leaf.keys().any(GroupKey::is_non_string),
            Tree::Branch(branch) => {
                branch.keys().any(GroupKey::is_non_string)
                    || branch.values().any(Self::has_non_string_key)
            }
        }
    }

    fn into_table(self, column_names: &[String], head: Span) -> Value {
        self._into_table(head)
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .rev()
                    .zip(column_names)
                    .map(|(val, key)| (key.clone(), val))
                    .collect::<Record>()
                    .into_value(head)
            })
            .collect::<Vec<_>>()
            .into_value(head)
    }

    fn _into_table(self, head: Span) -> Vec<Vec<Value>> {
        match self.groups {
            Tree::Leaf(leaf) => leaf
                .into_iter()
                .map(|(group, values)| vec![values.into_value(head), group.into_value(head)])
                .collect::<Vec<Vec<Value>>>(),
            Tree::Branch(branch) => branch
                .into_iter()
                .flat_map(|(group, items)| {
                    let group_val = group.into_value(head);
                    let mut inner = items._into_table(head);
                    for row in &mut inner {
                        row.push(group_val.clone());
                    }
                    inner
                })
                .collect(),
        }
    }

    fn into_record(self, head: Span) -> Value {
        match self.groups {
            Tree::Leaf(leaf) => Value::record(
                leaf.into_iter()
                    // Records cannot use null as a key; omit null groups rather than
                    // mapping them to the empty string (which collides with "").
                    .filter_map(|(k, v)| {
                        k.as_record_key()
                            .map(|key| (key.to_string(), v.into_value(head)))
                    })
                    .collect(),
                head,
            ),
            Tree::Branch(branch) => {
                let values = branch
                    .into_iter()
                    .filter_map(|(k, v)| {
                        k.as_record_key()
                            .map(|key| (key.to_string(), v.into_record(head)))
                    })
                    .collect();
                Value::record(values, head)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(GroupBy)
    }
}
