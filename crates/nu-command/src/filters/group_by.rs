use indexmap::IndexMap;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::{engine::Closure, IntoValue};

#[derive(Clone)]
pub struct GroupBy;

impl Command for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by")
            // TODO: It accepts Table also, but currently there is no Table
            // example. Perhaps Table should be a subtype of List, in which case
            // the current signature would suffice even when a Table example
            // exists.
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Any)])
            .switch(
                "to-table",
                "Return a table with \"groups\" and \"items\" columns",
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
        r#"the group-by command makes some assumptions:
    - if the input data is not a string, the grouper will convert the key to string but the values will remain in their original format. e.g. with bools, "true" and true would be in the same group (see example).
    - datetime is formatted based on your configuration setting. use `format date` to change the format.
    - filesize is formatted based on your configuration setting. use `format filesize` to change the format.
    - some nushell values are not supported, such as closures."#
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group items by the \"type\" column's values",
                example: r#"ls | group-by type"#,
                result: None,
            },
            Example {
                description: "Group items by the \"foo\" column's values, ignoring records without a \"foo\" column",
                example: r#"open cool.json | group-by foo?"#,
                result: None,
            },
            Example {
                description: "Group using a block which is evaluated against each input value",
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
                description: "You can also group by raw values by leaving out the argument",
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
                description: "You can also output a table instead of a record",
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
                description: "Group bools, whether they are strings or actual bools",
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
                description: "Group items by multiple columns' values",
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
                }))
            },
            Example {
                description: "Group items by multiple columns' values",
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
                ]))
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
    let groupers: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let to_table = call.has_flag(engine_state, stack, "to-table")?;
    let config = engine_state.get_config();

    let values: Vec<Value> = input.into_iter().collect();
    if values.is_empty() {
        return Ok(Value::record(Record::new(), head).into_pipeline_data());
    }

    let mut closure_idx = 0;

    // Error early on unsupported types by collecting
    let groupers = groupers
        .into_iter()
        .map(|val| match val {
            Value::CellPath { val, .. } => Ok(Grouper::CellPath { val }),
            Value::Closure {
                val, internal_span, ..
            } => {
                closure_idx += 1;
                Ok(Grouper::Closure {
                    val,
                    idx: closure_idx - 1,
                    span: internal_span,
                })
            }
            _ => Err(ShellError::TypeMismatch {
                err_message: "unsupported grouper type".to_string(),
                span: val.span(),
            }),
        })
        .collect::<Result<Vec<_>, ShellError>>()?;
    let grouped = match &groupers[..] {
        [first, rest @ ..] => {
            let mut grouped = Grouped::new(first, values, config, engine_state, stack)?;
            for grouper in rest {
                grouped.subgroup(grouper, config, engine_state, stack)?;
            }
            grouped
        }
        [] => Grouped::empty(values, config),
    };

    let value = if to_table {
        grouped.into_table(head)
    } else {
        grouped.into_record(head)
    };

    Ok(value.into_pipeline_data())
}

fn group_cell_path(
    column_name: &CellPath,
    values: Vec<Value>,
    config: &nu_protocol::Config,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();

    for value in values.into_iter() {
        let key = value
            .clone()
            .follow_cell_path(&column_name.members, false)?;

        if matches!(key, Value::Nothing { .. }) {
            continue; // likely the result of a failed optional access, ignore this value
        }

        let key = key.to_abbreviated_string(config);
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
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let mut closure = ClosureEval::new(engine_state, stack, closure);
    let config = engine_state.get_config();

    for value in values {
        let key = closure
            .run_with_value(value.clone())?
            .into_value(span)?
            .to_abbreviated_string(config);

        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

enum Grouper {
    CellPath {
        val: CellPath,
    },
    Closure {
        val: Box<Closure>,
        span: Span,
        idx: usize,
    },
}

impl Grouper {
    fn to_column_name(&self) -> String {
        match self {
            Grouper::CellPath { val, .. } => val.to_column_name(),
            Grouper::Closure { idx, .. } => format!("closure_{idx}"),
        }
    }
}

struct Grouped {
    grouper: String,
    groups: Tree,
}

enum Tree {
    Leaf(IndexMap<String, Vec<Value>>),
    Branch(IndexMap<String, Grouped>),
}

impl Grouped {
    fn empty(values: Vec<Value>, config: &nu_protocol::Config) -> Self {
        let mut groups = IndexMap::<_, Vec<_>>::new();

        for value in values.into_iter() {
            let key = value.to_abbreviated_string(config);
            groups.entry(key).or_default().push(value);
        }

        Self {
            grouper: "group".into(),
            groups: Tree::Leaf(groups),
        }
    }

    fn new(
        grouper: &Grouper,
        values: Vec<Value>,
        config: &nu_protocol::Config,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Self, ShellError> {
        let groups = match grouper {
            Grouper::CellPath { val, .. } => group_cell_path(val, values, config)?,
            Grouper::Closure { val, span, .. } => {
                group_closure(values, *span, Closure::clone(val), engine_state, stack)?
            }
        };
        let grouper = grouper.to_column_name();
        Ok(Self {
            grouper,
            groups: Tree::Leaf(groups),
        })
    }

    fn subgroup(
        &mut self,
        grouper: &Grouper,
        config: &nu_protocol::Config,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<(), ShellError> {
        let groups = match &mut self.groups {
            Tree::Leaf(groups) => std::mem::take(groups)
                .into_iter()
                .map(|(key, values)| -> Result<_, ShellError> {
                    let leaf = Self::new(grouper, values, config, engine_state, stack)?;
                    Ok((key, leaf))
                })
                .collect::<Result<IndexMap<_, _>, ShellError>>()?,
            Tree::Branch(nested_groups) => {
                let mut nested_groups = std::mem::take(nested_groups);
                for v in nested_groups.values_mut() {
                    v.subgroup(grouper, config, engine_state, stack)?;
                }
                nested_groups
            }
        };
        self.groups = Tree::Branch(groups);
        Ok(())
    }

    fn into_table(self, head: Span) -> Value {
        self._into_table(head)
            .into_iter()
            .map(|row| row.into_iter().rev().collect::<Record>().into_value(head))
            .collect::<Vec<_>>()
            .into_value(head)
    }

    fn _into_table(self, head: Span) -> Vec<Record> {
        match self.groups {
            Tree::Leaf(leaf) => leaf
                .into_iter()
                .map(|(group, values)| {
                    [
                        ("items".to_string(), values.into_value(head)),
                        (self.grouper.clone(), group.into_value(head)),
                    ]
                    .into_iter()
                    .collect()
                })
                .collect::<Vec<Record>>(),
            Tree::Branch(branch) => branch
                .into_iter()
                .flat_map(|(group, items)| {
                    let mut inner = items._into_table(head);
                    for row in &mut inner {
                        row.insert(self.grouper.clone(), group.clone().into_value(head));
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
                    .map(|(k, v)| (k, v.into_value(head)))
                    .collect(),
                head,
            ),
            Tree::Branch(branch) => {
                let values = branch
                    .into_iter()
                    .map(|(k, v)| (k, v.into_record(head)))
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(GroupBy {})
    }
}
