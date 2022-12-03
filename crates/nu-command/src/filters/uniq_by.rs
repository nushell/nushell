pub use super::uniq;
use nu_engine::column::nonexistent_column;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct UniqBy;

impl Command for UniqBy {
    fn name(&self) -> &str {
        "uniq-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq-by")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .rest("columns", SyntaxShape::Any, "the column(s) to filter by")
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
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;

        if columns.is_empty() {
            return Err(ShellError::MissingParameter("columns".into(), call.head));
        }

        let metadata = input.metadata();

        let vec: Vec<_> = input.into_iter().collect();
        match validate(vec.clone(), &columns, call.head) {
            Ok(_) => {}
            Err(err) => {
                return Err(err);
            }
        }

        let mapper = Box::new(item_mapper_by_col(columns));

        uniq(engine_state, stack, call, vec, mapper, metadata)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get rows from table filtered by column uniqueness ",
            example: "[[fruit count]; [apple 9] [apple 2] [pear 3] [orange 7]] | uniq-by fruit",
            result: Some(Value::List {
                vals: vec![
                    Value::test_record(
                        vec!["fruit", "count"],
                        vec![Value::test_string("apple"), Value::test_int(9)],
                    ),
                    Value::test_record(
                        vec!["fruit", "count"],
                        vec![Value::test_string("pear"), Value::test_int(3)],
                    ),
                    Value::test_record(
                        vec!["fruit", "count"],
                        vec![Value::test_string("orange"), Value::test_int(7)],
                    ),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn validate(vec: Vec<Value>, columns: &Vec<String>, span: Span) -> Result<(), ShellError> {
    if vec.is_empty() {
        return Err(ShellError::GenericError(
            "no values to work with".to_string(),
            "".to_string(),
            None,
            Some("no values to work with".to_string()),
            Vec::new(),
        ));
    }

    if let Value::Record {
        cols,
        vals: _input_vals,
        span: val_span,
    } = &vec[0]
    {
        if columns.is_empty() {
            // This uses the same format as the 'requires a column name' error in split_by.rs
            return Err(ShellError::GenericError(
                "expected name".into(),
                "requires a column name to filter table data".into(),
                Some(span),
                None,
                Vec::new(),
            ));
        }

        if let Some(nonexistent) = nonexistent_column(columns.clone(), cols.to_vec()) {
            return Err(ShellError::CantFindColumn(nonexistent, span, *val_span));
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

        let col_vals = Value::List {
            vals: item_column_values,
            span: Span::unknown(),
        };

        crate::ValueCounter::new_vals_to_compare(ms.item, ms.flag_ignore_case, col_vals)
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
