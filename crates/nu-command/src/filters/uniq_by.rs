pub use super::uniq;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{ast::PathMember, casing::Casing};

#[derive(Clone)]
pub struct UniqBy;

impl Command for UniqBy {
    fn name(&self) -> &str {
        "uniq-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq-by")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .rest(
                "columns",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The cell path(s) or closure(s) to filter by.",
            )
            .switch(
                "count",
                "Return a table containing the distinct input values together with their counts.",
                Some('c'),
            )
            .switch(
                "keep-last",
                "Keep the last occurrence of each unique value, in the position of its first occurrence.",
                Some('l'),
            )
            .switch(
                "repeated",
                "Return the input values that occur more than once.",
                Some('d'),
            )
            .switch(
                "ignore-case",
                "Ignore differences in case when comparing input values.",
                Some('i'),
            )
            .switch(
                "unique",
                "Return the input values that occur once only.",
                Some('u'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Return the distinct values in the input by the given column(s)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["distinct", "deduplicate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let key_vals: Vec<Value> = call.rest(engine_state, stack, 0)?;

        if key_vals.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "columns".into(),
                span: head,
            });
        }

        let keys = key_vals
            .into_iter()
            .map(|val| {
                let span = val.span();
                match val {
                    Value::CellPath { val, .. } => Ok(UniqKey::CellPath(val)),
                    // Strings coming from variables are treated as a single column name.
                    Value::String { val, .. } => Ok(UniqKey::CellPath(CellPath {
                        members: vec![PathMember::string(val, false, Casing::Sensitive, span)],
                    })),
                    Value::Closure { val, .. } => Ok(UniqKey::Closure(Box::new(
                        ClosureEval::try_new(engine_state, stack, *val, head)?,
                    ))),
                    _ => Err(ShellError::TypeMismatch {
                        err_message: "Cannot use a value which is not a cell path or closure"
                            .into(),
                        span,
                    }),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let metadata = input.take_metadata();

        let mapper = Box::new(item_mapper_by_keys(keys));

        let vec: Vec<_> = input.into_iter().collect();
        uniq(engine_state, stack, call, vec, mapper, metadata)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get rows from table filtered by column uniqueness.",
                example: "[[fruit count]; [apple 9] [apple 2] [pear 3] [orange 7]] | uniq-by fruit",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "fruit" => Value::test_string("apple"),
                        "count" => Value::test_int(9),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("pear"),
                        "count" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("orange"),
                        "count" => Value::test_int(7),
                    }),
                ])),
            },
            Example {
                description: "Get rows from table filtered by column uniqueness, keeping the last occurrence of each duplicate.",
                example: "[[fruit count]; [apple 9] [apple 2] [pear 3] [orange 7]] | uniq-by fruit --keep-last",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "fruit" => Value::test_string("apple"),
                        "count" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("pear"),
                        "count" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("orange"),
                        "count" => Value::test_int(7),
                    }),
                ])),
            },
            Example {
                description: "Get rows from table filtered by the uniqueness of a nested column.",
                example: "[{a: {b: 1} c: x} {a: {b: 1} c: y} {a: {b: 2} c: z}] | uniq-by a.b",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_record(record! { "b" => Value::test_int(1) }),
                        "c" => Value::test_string("x"),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_record(record! { "b" => Value::test_int(2) }),
                        "c" => Value::test_string("z"),
                    }),
                ])),
            },
            Example {
                description: "Get values filtered by the uniqueness of the result of a closure.",
                example: "[1 2 3 4 5] | uniq-by { $in mod 2 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
        ]
    }
}

enum UniqKey {
    CellPath(CellPath),
    Closure(Box<ClosureEval>),
}

fn item_mapper_by_keys(
    mut keys: Vec<UniqKey>,
) -> impl FnMut(crate::ItemMapperState) -> Result<crate::ValueCounter, ShellError> {
    move |ms: crate::ItemMapperState| -> Result<crate::ValueCounter, ShellError> {
        // Resolve each requested key while building the comparison value.
        // Validation and extraction share the same access semantics.
        let item_column_values = keys
            .iter_mut()
            .map(|key| match key {
                UniqKey::CellPath(path) => ms
                    .item
                    .follow_cell_path(&path.members)
                    .map(|value| value.into_owned()),
                UniqKey::Closure(closure) => closure
                    .run_with_value(ms.item.clone())?
                    .into_value(ms.head),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let col_vals = Value::list(item_column_values, ms.head);

        Ok(crate::ValueCounter::new_vals_to_compare(
            ms.item,
            ms.flag_ignore_case,
            col_vals,
            ms.index,
            ms.head,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(UniqBy)
    }
}
