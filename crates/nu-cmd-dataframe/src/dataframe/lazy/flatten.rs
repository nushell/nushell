use super::explode::explode;
use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct LazyFlatten;

impl Command for LazyFlatten {
    fn name(&self) -> &str {
        "dfr flatten"
    }

    fn usage(&self) -> &str {
        "An alias for dfr explode."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "columns",
                SyntaxShape::String,
                "columns to flatten, only applicable for dataframes",
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
                description: "Flatten the specified dataframe",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | dfr into-df | dfr flatten hobbies | dfr collect",
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
                description: "Select a column and flatten the values",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | dfr into-df | dfr select (dfr col hobbies | dfr flatten)",
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

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(LazyFlatten {})]);
        test_dataframe_example(&mut engine_state, &LazyFlatten.examples()[0]);
    }

    #[ignore]
    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(LazyFlatten {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &LazyFlatten.examples()[1]);
    }
}
