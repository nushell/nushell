use crate::dataframe::values::{NuExpression, NuLazyFrame, NuLazyGroupBy};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
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

        let group_by = NuLazyGroupBy::try_from_pipeline(input, call.head)?;
        let from_eager = group_by.from_eager;

        let group_by = group_by.into_polars();
        let lazy: NuLazyFrame = group_by.agg(&expressions).into();

        let res = if from_eager {
            lazy.collect(call.head)?.into_value(call.head)
        } else {
            lazy.into_value(call.head)
        };

        Ok(PipelineData::Value(res, None))
    }
}
