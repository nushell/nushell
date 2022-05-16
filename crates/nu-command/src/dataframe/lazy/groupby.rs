use crate::dataframe::values::{NuExpression, NuLazyFrame, NuLazyGroupBy};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct ToLazyGroupBy;

impl Command for ToLazyGroupBy {
    fn name(&self) -> &str {
        "dfr group-by"
    }

    fn usage(&self) -> &str {
        "Creates a groupby object that can be used for other aggregations"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the lazy group by",
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
                result: None,
            },
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr to-df
    | dfr to-lazy
    | dfr group-by a
    | dfr aggregate [
        ("b" | dfr min | dfr as "b_min")
        ("b" | dfr max | dfr as "b_max")
        ("b" | dfr sum | dfr as "b_sum")
     ]
    | dfr collect"#,
                result: None,
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

        if expressions
            .iter()
            .any(|expr| !matches!(expr, Expr::Column(..)))
        {
            let value: Value = call.req(engine_state, stack, 0)?;
            return Err(ShellError::IncompatibleParametersSingle(
                "Expected only Col expressions".into(),
                value.span()?,
            ));
        }

        let value = input.into_value(call.head);
        let (lazy, from_eager) = NuLazyFrame::maybe_is_eager(value)?;

        let group_by = NuLazyGroupBy {
            group_by: Some(lazy.into_polars().groupby(&expressions)),
            from_eager,
        };

        Ok(PipelineData::Value(group_by.into_value(call.head), None))
    }
}
