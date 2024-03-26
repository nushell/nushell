use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame, NuLazyGroupBy};
use nu_engine::command_prelude::*;

use polars::prelude::Expr;

#[derive(Clone)]
pub struct ToLazyGroupBy;

impl Command for ToLazyGroupBy {
    fn name(&self) -> &str {
        "dfr group-by"
    }

    fn usage(&self) -> &str {
        "Creates a group-by object that can be used for other aggregations."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group-by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the lazy group-by",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-df
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(2)],
                            ),
                            Column::new(
                                "b_min".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "b_max".to_string(),
                                vec![Value::test_int(4), Value::test_int(6)],
                            ),
                            Column::new(
                                "b_sum".to_string(),
                                vec![Value::test_int(6), Value::test_int(10)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-lazy
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]
    | dfr collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(2)],
                            ),
                            Column::new(
                                "b_min".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "b_max".to_string(),
                                vec![Value::test_int(4), Value::test_int(6)],
                            ),
                            Column::new(
                                "b_sum".to_string(),
                                vec![Value::test_int(6), Value::test_int(10)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(value)?;

        if expressions
            .iter()
            .any(|expr| !matches!(expr, Expr::Column(..)))
        {
            let value: Value = call.req(engine_state, stack, 0)?;
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Expected only Col expressions".into(),
                span: value.span(),
            });
        }

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let group_by = NuLazyGroupBy {
            schema: lazy.schema.clone(),
            from_eager: lazy.from_eager,
            group_by: Some(lazy.into_polars().group_by(&expressions)),
        };

        Ok(PipelineData::Value(group_by.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::{ExprAlias, ExprMax, ExprMin, ExprSum};
    use crate::dataframe::lazy::aggregate::LazyAggregate;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
            Box::new(ExprAlias {}),
            Box::new(ExprMin {}),
            Box::new(ExprMax {}),
            Box::new(ExprSum {}),
        ])
    }
}
