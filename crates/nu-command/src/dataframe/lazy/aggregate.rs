use crate::dataframe::values::{NuExpression, NuLazyFrame, NuLazyGroupBy, NuDataFrame, Column};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value, Span,
};

#[derive(Clone)]
pub struct LazyAggregate;

impl Command for LazyAggregate {
    fn name(&self) -> &str {
        "dfr aggregate"
    }

    fn usage(&self) -> &str {
        "Performs a series of aggregations from a group by"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the aggregations to be applied",
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr to-df
    | dfr group-by a
    | dfr aggregate [
        ("b" | dfr min | dfr as "b_min")
        ("b" | dfr max | dfr as "b_max")
        ("b" | dfr sum | dfr as "b_sum")
     ]"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr to-lazy
    | dfr group-by a
    | dfr aggregate [
        ("b" | dfr min | dfr as "b_min")
        ("b" | dfr max | dfr as "b_max")
        ("b" | dfr sum | dfr as "b_sum")
     ]
    | dfr collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
                    ])
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
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = NuExpression::extract_exprs(value)?;

        let group_by = NuLazyGroupBy::try_from_pipeline(input, call.head)?;
        let from_eager = group_by.from_eager;
        
        let group_by = group_by.into_polars();
        let lazy = NuLazyFrame {
            lazy: group_by.agg(&expressions).into(),
            from_eager
        };

        let res = lazy.into_value(call.head)?;
        Ok(PipelineData::Value(res, None))
    }
}

#[cfg(test)]
mod test {
    use crate::dataframe::expressions::ExprAlias;
    use crate::dataframe::lazy::{LazyMin, LazyMax, LazySum};
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(LazyAggregate {}), 
            Box::new(ToLazyGroupBy {}),
            Box::new(ExprAlias {}),
            Box::new(LazyMin {}),
            Box::new(LazyMax {}),
            Box::new(LazySum {}),
        ])
    }
}
