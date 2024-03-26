use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct LazyExplode;

impl Command for LazyExplode {
    fn name(&self) -> &str {
        "dfr explode"
    }

    fn usage(&self) -> &str {
        "Explodes a dataframe or creates a explode expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "columns",
                SyntaxShape::String,
                "columns to explode, only applicable for dataframes",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Explode the specified dataframe",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | dfr into-df | dfr explode hobbies | dfr collect",
                result: Some(
                   NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "id".to_string(),
                        vec![
                            Value::test_int(1),
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(2),
                        ]),
                    Column::new(
                        "name".to_string(),
                        vec![
                            Value::test_string("Mercy"),
                            Value::test_string("Mercy"),
                            Value::test_string("Bob"),
                            Value::test_string("Bob"),
                        ]),
                    Column::new(
                        "hobbies".to_string(),
                        vec![
                            Value::test_string("Cycling"),
                            Value::test_string("Knitting"),
                            Value::test_string("Skiing"),
                            Value::test_string("Football"),
                        ]),
                   ], None).expect("simple df for test should not fail")
                   .into_value(Span::test_data()),
                    )
            },
            Example {
                description: "Select a column and explode the values",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | dfr into-df | dfr select (dfr col hobbies | dfr explode)",
                result: Some(
                   NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "hobbies".to_string(),
                        vec![
                            Value::test_string("Cycling"),
                            Value::test_string("Knitting"),
                            Value::test_string("Skiing"),
                            Value::test_string("Football"),
                        ]),
                   ], None).expect("simple df for test should not fail")
                   .into_value(Span::test_data()),
                    ),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        explode(call, input)
    }
}

pub(crate) fn explode(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head);
    if NuDataFrame::can_downcast(&value) {
        let df = NuLazyFrame::try_from_value(value)?;
        let columns: Vec<String> = call
            .positional_iter()
            .filter_map(|e| e.as_string())
            .collect();

        let exploded = df
            .into_polars()
            .explode(columns.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

        Ok(PipelineData::Value(
            NuLazyFrame::from(exploded).into_value(call.head)?,
            None,
        ))
    } else {
        let expr = NuExpression::try_from_value(value)?;
        let expr: NuExpression = expr.into_polars().explode().into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(LazyExplode {})]);
        test_dataframe_example(&mut engine_state, &LazyExplode.examples()[0]);
    }

    #[ignore]
    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(LazyExplode {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &LazyExplode.examples()[1]);
    }
}
