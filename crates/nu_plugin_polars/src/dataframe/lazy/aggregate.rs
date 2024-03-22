use crate::{
    dataframe::values::{NuExpression, NuLazyFrame, NuLazyGroupBy},
    values::{Column, NuDataFrame},
    PolarsDataFramePlugin,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};
use polars::{datatypes::DataType, prelude::Expr};

#[derive(Clone)]
pub struct LazyAggregate;

impl PluginCommand for LazyAggregate {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars agg")
            .usage("Performs a series of aggregations from a group-by.")
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
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let vals: Vec<Value> = call.rest(0)?;
        let value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(value)?;

        let group_by = NuLazyGroupBy::try_from_pipeline(input, call.head)?;

        if let Some(schema) = &group_by.schema {
            for expr in expressions.iter() {
                if let Some(name) = get_col_name(expr) {
                    let dtype = schema.get(name.as_str());

                    if matches!(dtype, Some(DataType::Object(..))) {
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
        }

        let polars = group_by.into_polars();

        let lazy = NuLazyFrame::new(group_by.from_eager, polars.agg(&expressions));

        let res = lazy.into_value(call.head)?;
        Ok(PipelineData::Value(res, None))
    }
}

fn examples() -> Vec<PluginExample> {
    vec![
        PluginExample {
            description: "Group by and perform an aggregation".into(),
            example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | polars into-df
    | polars group-by a
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
     ]"#
            .into(),
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
        PluginExample {
            description: "Group by and perform an aggregation".into(),
            example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | polars into-lazy
    | polars group-by a
    | polars agg [
        (polars col b | polars min | polars as "b_min")
        (polars col b | polars max | polars as "b_max")
        (polars col b | polars sum | polars as "b_sum")
     ]
    | polars collect"#
                .into(),
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
            | polars::prelude::AggExpr::Count(e, _)
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
        | Expr::Exclude(expr, _)
        | Expr::Alias(expr, _)
        | Expr::KeepName(expr)
        | Expr::Explode(expr) => get_col_name(expr.as_ref()),
        Expr::Ternary { .. }
        | Expr::AnonymousFunction { .. }
        | Expr::Function { .. }
        | Expr::Columns(_)
        | Expr::DtypeColumn(_)
        | Expr::Literal(_)
        | Expr::BinaryExpr { .. }
        | Expr::Window { .. }
        | Expr::Wildcard
        | Expr::RenameAlias { .. }
        | Expr::Len
        | Expr::Nth(_)
        | Expr::SubPlan(_, _)
        | Expr::Selector(_) => None,
    }
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::expressions::{ExprAlias, ExprMax, ExprMin, ExprSum};
//     use crate::dataframe::lazy::groupby::ToLazyGroupBy;
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
