pub use super::uniq;
use nu_engine::{column::nonexistent_column, command_prelude::*};

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
                "Return a table containing the distinct input values together with their counts",
                Some('c'),
            )
            .switch(
                "repeated",
                "Return the input values that occur more than once",
                Some('d'),
            )
            .switch(
                "ignore-case",
                "Ignore differences in case when comparing input values",
                Some('i'),
            )
            .switch(
                "unique",
                "Return the input values that occur once only",
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
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;

        if columns.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "columns".into(),
                span: call.head,
            });
        }

        let metadata = input.metadata();

        let vec: Vec<_> = input.into_iter().collect();
        match validate(&vec, &columns, call.head) {
            Ok(_) => {}
            Err(err) => {
                return Err(err);
            }
        }

        let mapper = Box::new(item_mapper_by_col(columns));

        uniq(engine_state, stack, call, vec, mapper, metadata)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get rows from table filtered by column uniqueness ",
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
        }]
    }
}

fn validate(vec: &[Value], columns: &[String], span: Span) -> Result<(), ShellError> {
    let first = vec.first();
    if let Some(v) = first {
        let val_span = v.span();
        if let Value::Record { val: record, .. } = &v {
            if columns.is_empty() {
                return Err(ShellError::GenericError {
                    error: "expected name".into(),
                    msg: "requires a column name to filter table data".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }

            if let Some(nonexistent) = nonexistent_column(columns, record.columns()) {
                return Err(ShellError::CantFindColumn {
                    col_name: nonexistent,
                    span: Some(span),
                    src_span: val_span,
                });
            }
        }
    }

    Ok(())
}

fn get_data_by_columns(columns: &[String], item: &Value) -> Vec<Value> {
    columns
        .iter()
        .filter_map(|col| item.get_data_by_key(col))
        .collect::<Vec<_>>()
}

fn item_mapper_by_col(cols: Vec<String>) -> impl Fn(crate::ItemMapperState) -> crate::ValueCounter {
    let columns = cols;

    Box::new(move |ms: crate::ItemMapperState| -> crate::ValueCounter {
        let item_column_values = get_data_by_columns(&columns, &ms.item);

        let col_vals = Value::list(item_column_values, Span::unknown());

        crate::ValueCounter::new_vals_to_compare(ms.item, ms.flag_ignore_case, col_vals, ms.index)
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UniqBy {})
    }
}
