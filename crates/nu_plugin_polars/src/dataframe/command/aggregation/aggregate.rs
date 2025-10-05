use crate::{
    PolarsPlugin,
    dataframe::values::{NuExpression, NuLazyFrame, NuLazyGroupBy},
    values::{Column, CustomValueSupport, NuDataFrame},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::{datatypes::DataType, prelude::Expr};

#[derive(Clone)]
pub struct LazyAggregate;

impl PluginCommand for LazyAggregate {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars agg"
    }

    fn description(&self) -> &str {
        "Performs a series of aggregations from a group-by."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group-by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the aggregations to be applied",
            )
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
                description: "Group by and perform an aggregation using a record",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
                | polars into-lazy
                | polars group-by a
                | polars agg {
                    b_min: (polars col b | polars min)
                    b_max: (polars col b | polars max)
                    b_sum: (polars col b | polars sum)
                 }
                | polars collect
                | polars sort-by a"#,
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
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let vals: Vec<Value> = call.rest(0)?;
        let value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, value)?;

        let group_by = NuLazyGroupBy::try_from_pipeline(plugin, input, call.head)?;

        for expr in expressions.iter() {
            if let Some(name) = get_col_name(expr) {
                let dtype = group_by.schema.schema.get(name.as_str());

                if let Some(DataType::Object(..)) = dtype {
                    return Err(ShellError::GenericError {
                            error: "Object type column not supported for aggregation".into(),
                            msg: format!("Column '{name}' is type Object"),
                            span: Some(call.head),
                            help: Some("Aggregations cannot be performed on Object type columns. Use dtype command to check column types".into()),
                            inner: vec![],
                        }).map_err(|e| e.into());
                }
            }
        }

        let polars = group_by.to_polars();
        let lazy = NuLazyFrame::new(false, polars.agg(&expressions));
        lazy.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn get_col_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Column(column) => Some(column.to_string()),
        Expr::Agg(agg) => match agg {
            polars::prelude::AggExpr::Min { input: e, .. }
            | polars::prelude::AggExpr::Max { input: e, .. }
            | polars::prelude::AggExpr::Median(e)
            | polars::prelude::AggExpr::NUnique(e)
            | polars::prelude::AggExpr::First(e)
            | polars::prelude::AggExpr::Last(e)
            | polars::prelude::AggExpr::Mean(e)
            | polars::prelude::AggExpr::Implode(e)
            | polars::prelude::AggExpr::Count { input: e, .. }
            | polars::prelude::AggExpr::Sum(e)
            | polars::prelude::AggExpr::AggGroups(e)
            | polars::prelude::AggExpr::Std(e, _)
            | polars::prelude::AggExpr::Var(e, _) => get_col_name(e.as_ref()),
            polars::prelude::AggExpr::Quantile { expr, .. } => get_col_name(expr.as_ref()),
        },
        Expr::Filter { input: expr, .. }
        | Expr::Slice { input: expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Sort { expr, .. }
        | Expr::Gather { expr, .. }
        | Expr::SortBy { expr, .. }
        | Expr::KeepName(expr)
        | Expr::Explode { input: expr, .. } => get_col_name(expr.as_ref()),
        Expr::Ternary { .. }
        | Expr::AnonymousFunction { .. }
        | Expr::Function { .. }
        | Expr::Literal(_)
        | Expr::BinaryExpr { .. }
        | Expr::Window { .. }
        | Expr::RenameAlias { .. }
        | Expr::Len
        | Expr::SubPlan(_, _)
        | Expr::Selector(_)
        | Expr::Field(_)
        | Expr::Alias(_, _)
        | Expr::DataTypeFunction(_)
        | Expr::Eval { .. } => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyAggregate)
    }
}
