use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame, NuLazyGroupBy};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::{datatypes::DataType, prelude::Expr};

#[derive(Clone)]
pub struct LazyAggregate;

impl Command for LazyAggregate {
    fn name(&self) -> &str {
        "agg"
    }

    fn usage(&self) -> &str {
        "Performs a series of aggregations from a group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "Group-by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the aggregations to be applied",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group by and perform an aggregation",
                example: r#"[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | into df
    | group-by a
    | agg [
        (col b | min | as "b_min")
        (col b | max | as "b_max")
        (col b | sum | as "b_sum")
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
    | into lazy
    | group-by a
    | agg [
        (col b | min | as "b_min")
        (col b | max | as "b_max")
        (col b | sum | as "b_sum")
     ]
    | collect"#,
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

        if let Some(schema) = &group_by.schema {
            for expr in &expressions {
                if let Some(name) = get_col_name(expr) {
                    let dtype = schema.get(name.as_str());

                    if matches!(dtype, Some(DataType::Object(..))) {
                        return Err(ShellError::GenericError(
                            "Object type column not supported for aggregation".into(),
                            format!("Column '{}' is type Object", name),
                            Some(call.head),
                            Some("Aggregations cannot be performed on Object type columns. Use dtype command to check column types".into()),
                            Vec::new(),
                        ));
                    }
                }
            }
        }

        let lazy = NuLazyFrame {
            from_eager: group_by.from_eager,
            lazy: Some(group_by.into_polars().agg(&expressions)),
            schema: None,
        };

        let res = lazy.into_value(call.head)?;
        Ok(PipelineData::Value(res, None))
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
            | polars::prelude::AggExpr::List(e)
            | polars::prelude::AggExpr::Count(e)
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
        | Expr::Take { expr, .. }
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
        | Expr::Count
        | Expr::Nth(_) => None,
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::expressions::{ExprAlias, ExprMax, ExprMin, ExprSum};
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

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
