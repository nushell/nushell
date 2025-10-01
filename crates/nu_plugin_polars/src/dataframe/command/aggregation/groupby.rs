use crate::{
    PolarsPlugin,
    dataframe::values::{NuDataFrame, NuExpression, NuLazyFrame, NuLazyGroupBy},
    values::CustomValueSupport,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::{df, prelude::Expr};

#[derive(Clone)]
pub struct ToLazyGroupBy;

impl PluginCommand for ToLazyGroupBy {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars group-by"
    }

    fn description(&self) -> &str {
        "Creates a group-by object that can be used for other aggregations."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group-by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the lazy group-by",
            )
            .switch(
                "maintain-order",
                "Ensure that the order of the groups is consistent with the input data. This is slower than a default group by and cannot be run on the streaming engine.",
                Some('m'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | polars into-lazy
    | polars group-by a
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
     ]
    | polars collect
    | polars sort-by a"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => &[1i64, 2],
                            "b_min" => &[2i64, 4],
                            "b_max" => &[4i64, 6],
                            "b_sum" => &[6i64, 10],
                        )
                        .expect("should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Group by an expression and perform an aggregation",
                example: r#"[[a b]; [2025-04-01 1] [2025-04-02 2] [2025-04-03 3] [2025-04-04 4]]
    | polars into-lazy
    | polars group-by (polars col a | polars get-day | $in mod 2)
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
     ]
    | polars collect
    | polars sort-by a"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => &[0i64, 1],
                            "b_min" => &[2i64, 1],
                            "b_max" => &[4i64, 3],
                            "b_sum" => &[6i64, 4],
                        )
                        .expect("should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let vals: Vec<Value> = call.rest(0)?;
        let expr_value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, expr_value)?;
        let maintain_order = call.has_flag("maintain-order")?;

        if expressions
            .iter()
            .any(|expr| matches!(expr, Expr::Agg(..) | Expr::Window { .. }))
        {
            let value: Value = call.req(0)?;
            Err(ShellError::IncompatibleParametersSingle {
                msg: "Cannot group by an aggregation or window expression".into(),
                span: value.span(),
            })?;
        }

        let pipeline_value = input.into_value(call.head)?;
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &pipeline_value)?;
        command(plugin, engine, call, lazy, expressions, maintain_order)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    mut lazy: NuLazyFrame,
    expressions: Vec<Expr>,
    maintain_order: bool,
) -> Result<PipelineData, ShellError> {
    let group_by = if maintain_order {
        lazy.to_polars().group_by_stable(expressions)
    } else {
        lazy.to_polars().group_by(expressions)
    };
    let group_by = NuLazyGroupBy::new(group_by, lazy.from_eager, lazy.schema().clone()?);
    group_by.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToLazyGroupBy)
    }
}
