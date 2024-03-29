use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame, NuLazyGroupBy},
    values::{
        cant_convert_err, to_pipeline_data, CustomValueSupport, PolarsPluginObject,
        PolarsPluginType,
    },
    Cacheable, PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct ToLazyGroupBy;

impl PluginCommand for ToLazyGroupBy {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars group-by"
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
    | polars into-df
    | polars group-by a
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
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
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
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
    | polars collect"#,
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
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
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
        let vals: Vec<Value> = call.rest(0)?;
        let expr_value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, expr_value)?;

        if expressions
            .iter()
            .any(|expr| !matches!(expr, Expr::Column(..)))
        {
            let value: Value = call.req(0)?;
            Err(ShellError::IncompatibleParametersSingle {
                msg: "Expected only Col expressions".into(),
                span: value.span(),
            })?;
        }

        let pipeline_value = input.into_value(call.head);

        match PolarsPluginObject::try_from_value(plugin, &pipeline_value)? {
            PolarsPluginObject::NuDataFrame(df) => {
                cmd_lazy(plugin, engine, call, df.lazy(), expressions)
            }
            PolarsPluginObject::NuLazyFrame(lazy) => {
                cmd_lazy(plugin, engine, call, lazy, expressions)
            }
            _ => Err(cant_convert_err(
                &pipeline_value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
        .map_err(LabeledError::from)
    }
}

fn cmd_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    expressions: Vec<Expr>,
) -> Result<PipelineData, ShellError> {
    let group_by = lazy.to_polars().group_by(expressions);
    let group_by = NuLazyGroupBy::new(group_by, lazy.from_eager, lazy.schema()?);
    to_pipeline_data(plugin, engine, call.head, group_by)
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::expressions::{ExprAlias, ExprMax, ExprMin, ExprSum};
//     use crate::dataframe::lazy::aggregate::LazyAggregate;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(LazyAggregate {}),
//             Box::new(ToLazyGroupBy {}),
//             Box::new(ExprAlias {}),
//             Box::new(ExprMin {}),
//             Box::new(ExprMax {}),
//             Box::new(ExprSum {}),
//         ])
//     }
// }
