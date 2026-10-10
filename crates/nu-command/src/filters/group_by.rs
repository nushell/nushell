use indexmap::IndexMap;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{
    Config, FromValue, ast::PathMember, engine::Closure, shell_error::generic::GenericError,
};
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
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
                "Return a table with a column per grouper and an \"items\" column. Group keys keep their type.",
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
        "Splits a list or table into groups, and returns a record containing those groups."
    }

    fn extra_description(&self) -> &str {
        r#"By default the output is a record. Record keys are strings, so each group key is converted to its display string, and values that display alike share a group: "true" and true, 1 and "1", and filesizes such as 1MB and 1.001MB. Dates and filesizes display according to your config. Records cannot use null as a key, so null groups are omitted.

With --to-table the output is a table with one column per grouper (named `group` when no grouper is given) and an `items` column. Grouping compares the values themselves: keys share a group only when they have the same type and the same value, so 1 and 1.0 are separate groups, as are "true" and true, and 1MB and 1.001MB. Records with the same fields in a different order share a group, as do dates at the same instant in different time zones. Group columns keep their original types, and null groups are kept. A grouper named `items` is rejected (use `{ get items }` or rename the column), and so are duplicate grouper names.

To choose how values are bucketed, convert them in a closure, for example `group-by { get modified | format date "%F" }`. Optional cell paths (e.g. `foo?`) skip rows where access yields null."#
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
                description: "Group by a non-string column, keeping the keys' original type.",
                example: "[{n: 1} {n: 2} {n: 1}] | group-by n --to-table",
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
                description: "Group bools, whether they are strings or actual bools.",
                example: r#"[true "true" false "false"] | group-by"#,
                result: Some(Value::test_record(record! {
                    "true" => Value::test_list(vec![
                        Value::test_bool(true),
                        Value::test_string("true"),
                    ]),
                    "false" => Value::test_list(vec![
                        Value::test_bool(false),
                        Value::test_string("false"),
                    ]),
                })),
            },
            Example {
                description: "Bools and strings are different keys in a table.",
                example: r#"[true "true" false "false"] | group-by --to-table"#,
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
    let config = stack.get_config(engine_state);

    // The flag alone picks the output: a table keyed by value, or a record keyed by display
    // string. Table column names are checked before grouping, so bad names fail on empty input
    // too, and before any grouper closure runs.
    let (key_mode, column_names) = if to_table {
        (KeyMode::Value, Some(groupers_to_column_names(&groupers)?))
    } else {
        (KeyMode::Display(&config), None)
    };

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
            let mut grouped =
                Grouped::new(first.as_ref(), prune, values, key_mode, engine_state, stack)?;
            for grouper in rest {
                grouped.subgroup(grouper.as_ref(), prune, key_mode, engine_state, stack)?;
            }
            grouped
        }
        [] => Grouped::empty(values, key_mode)?,
    };

    let value = match column_names {
        Some(column_names) => grouped.into_table(&column_names, head),
        None => grouped.into_record(head),
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

/// How a grouper's value becomes a [`GroupKey`].
#[derive(Clone, Copy)]
enum KeyMode<'a> {
    /// `--to-table`: the value itself, so keys keep their type.
    Value,
    /// Record output: the display string, because record keys are strings. Null stays null so
    /// it doesn't collide with `""`.
    Display(&'a Config),
}

/// A group key, compared with [`Value::strict_eq`] and hashed with `Value`'s [`Hash`].
///
/// In [`KeyMode::Value`], `1`, `1.0`, and `"1"` are three groups. In [`KeyMode::Display`], every
/// key is a string or null, so `1` and `"1"` share a group. Either way `null` stays distinct from
/// `""`. Record output omits `null` keys; table output keeps them.
#[derive(Debug, Clone)]
struct GroupKey(Value);

impl GroupKey {
    fn new(value: Cow<'_, Value>, mode: KeyMode) -> Self {
        match mode {
            KeyMode::Display(config) if !value.is_nothing() => Self(Value::string(
                value.to_expanded_string(", ", config),
                value.span(),
            )),
            _ => Self(value.into_owned()),
        }
    }

    /// The record key for this group, or `None` for a null group, which records omit.
    ///
    /// Record output uses [`KeyMode::Display`], so every key is a string or null.
    fn into_record_key(self) -> Option<String> {
        match self.0 {
            Value::String { val, .. } => Some(val),
            _ => None,
        }
    }
}

// Not derived: `Value`'s `PartialEq` is Nushell's loose `==` (`1 == 1.0`), which `Hash` does not
// follow.
impl PartialEq for GroupKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.strict_eq(&other.0)
    }
}

impl Eq for GroupKey {}

impl Hash for GroupKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
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
    key_mode: KeyMode,
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

        let key = GroupKey::new(key_val, key_mode);

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
    key_mode: KeyMode,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<IndexMap<GroupKey, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let mut closure = ClosureEval::new(engine_state, stack, closure);

    for value in values {
        let key_val = closure.run_with_value(value.clone())?.into_value(span)?;
        let key = GroupKey::new(Cow::Owned(key_val), key_mode);

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
    /// The keys are already distinct and are never looked up again, so a `Vec` keeps them
    /// without hashing or comparing them a second time.
    Branch(Vec<(GroupKey, Grouped)>),
}

impl Grouped {
    fn empty(values: Vec<Value>, key_mode: KeyMode) -> Result<Self, ShellError> {
        let mut groups = IndexMap::<_, Vec<_>>::new();

        for value in values.into_iter() {
            // An error in the input is raised rather than grouped under an error key.
            let value = value.unwrap_error()?;
            let key = GroupKey::new(Cow::Borrowed(&value), key_mode);
            groups.entry(key).or_default().push(value);
        }

        Ok(Self {
            groups: Tree::Leaf(groups),
        })
    }

    fn new(
        grouper: Spanned<&Grouper>,
        prune: bool,
        values: Vec<Value>,
        key_mode: KeyMode,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Self, ShellError> {
        let groups = match grouper.item {
            Grouper::CellPath { val } => group_cell_path(val, prune, values, key_mode)?,
            Grouper::Closure { val } => group_closure(
                values,
                grouper.span,
                Closure::clone(val),
                key_mode,
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
        key_mode: KeyMode,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<(), ShellError> {
        let groups = match &mut self.groups {
            Tree::Leaf(groups) => std::mem::take(groups)
                .into_iter()
                .map(|(key, values)| -> Result<_, ShellError> {
                    let leaf = Self::new(grouper, prune, values, key_mode, engine_state, stack)?;
                    Ok((key, leaf))
                })
                .collect::<Result<Vec<_>, ShellError>>()?,
            Tree::Branch(nested_groups) => {
                let mut nested_groups = std::mem::take(nested_groups);
                for (_, v) in &mut nested_groups {
                    v.subgroup(grouper, prune, key_mode, engine_state, stack)?;
                }
                nested_groups
            }
        };
        self.groups = Tree::Branch(groups);
        Ok(())
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
                .map(|(group, values)| vec![values.into_value(head), group.0])
                .collect::<Vec<Vec<Value>>>(),
            Tree::Branch(branch) => branch
                .into_iter()
                .flat_map(|(group, items)| {
                    let mut inner = items._into_table(head);
                    for row in &mut inner {
                        row.push(group.0.clone());
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
                    .filter_map(|(k, v)| Some((k.into_record_key()?, v.into_value(head))))
                    .collect(),
                head,
            ),
            Tree::Branch(branch) => {
                let values = branch
                    .into_iter()
                    .filter_map(|(k, v)| Some((k.into_record_key()?, v.into_record(head))))
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
