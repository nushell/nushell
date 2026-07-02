pub use super::uniq;
use nu_engine::command_prelude::*;
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
            .rest("columns", SyntaxShape::Any, "The column(s) to filter by.")
            .switch(
                "count",
                "Return a table containing the distinct input values together with their counts.",
                Some('c'),
            )
            .switch(
                "keep-last",
                "Return the last occurrence of each unique value instead of the first.",
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
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;

        if columns.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "columns".into(),
                span: call.head,
            });
        }

        let metadata = input.take_metadata();

        let columns = columns
            .into_iter()
            .map(|col| PathMember::string(col, false, Casing::Sensitive, call.head))
            .collect();
        let mapper = Box::new(item_mapper_by_col(columns));

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
        ]
    }
}

fn item_mapper_by_col(
    columns: Vec<PathMember>,
) -> impl Fn(crate::ItemMapperState) -> Result<crate::ValueCounter, ShellError> {
    move |ms: crate::ItemMapperState| -> Result<crate::ValueCounter, ShellError> {
        // Use the same cell-path access as `group-by` while building the comparison value.
        // This preserves `IncompatiblePathAccess` for non-record rows and `CantFindColumn`/`DidYouMean` for missing columns.
        let item_column_values = columns
            .iter()
            .map(|column| {
                ms.item
                    .follow_cell_path(std::slice::from_ref(column))
                    .map(|value| value.into_owned())
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
