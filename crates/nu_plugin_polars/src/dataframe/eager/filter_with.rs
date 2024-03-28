use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::LazyFrame;

use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::PolarsPluginObject,
    Cacheable, CustomValueSupport, PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct FilterWith;

impl PluginCommand for FilterWith {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars filter-with"
    }

    fn usage(&self) -> &str {
        "Filters dataframe using a mask or expression as reference."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "mask or expression",
                SyntaxShape::Any,
                "boolean mask used to filter data",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter dataframe using a bool mask",
                example: r#"let mask = ([true false] | polars into-df);
    [[a b]; [1 2] [3 4]] | polars into-df | polars filter-with $mask"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(1)]),
                            Column::new("b".to_string(), vec![Value::test_int(2)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Filter dataframe using an expression",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars filter-with ((polars col a) > 1)",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(3)]),
                            Column::new("b".to_string(), vec![Value::test_int(4)]),
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
        let value = input.into_value(call.head);

        match PolarsPluginObject::try_from_value(plugin, &value).map_err(LabeledError::from)? {
            PolarsPluginObject::NuDataFrame(df) => {
                command_eager(plugin, engine, call, df).map_err(LabeledError::from)
            }
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_lazy(plugin, engine, call, lazy).map_err(LabeledError::from)
            }
            _ => Err(LabeledError::new("Unsupported type: {value}")
                .with_label("Unsupported Type", call.head)),
        }
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mask_value: Value = call.req(0)?;
    let mask_span = mask_value.span();

    if NuExpression::can_downcast(&mask_value) {
        let expression = NuExpression::try_from_value(plugin, &mask_value)?;
        let lazy = df.lazy();
        let lazy = lazy.apply_with_expr(expression, LazyFrame::filter);

        Ok(PipelineData::Value(
            lazy.cache(plugin, engine)?.into_value(call.head),
            None,
        ))
    } else {
        let mask = NuDataFrame::try_from_value(plugin, &mask_value)?.as_series(mask_span)?;
        let mask = mask.bool().map_err(|e| ShellError::GenericError {
            error: "Error casting to bool".into(),
            msg: e.to_string(),
            span: Some(mask_span),
            help: Some("Perhaps you want to use a series with booleans as mask".into()),
            inner: vec![],
        })?;

        let df = df
            .as_ref()
            .filter(mask)
            .map_err(|e| ShellError::GenericError {
                error: "Error filtering dataframe".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: Some("The only allowed column types for dummies are String or Int".into()),
                inner: vec![],
            })?;
        let df = NuDataFrame::new(false, df);
        Ok(PipelineData::Value(
            df.cache(plugin, engine)?.into_value(call.head),
            None,
        ))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let expr: Value = call.req(0)?;
    let expr = NuExpression::try_from_value(plugin, &expr)?;

    let lazy = lazy.apply_with_expr(expr, LazyFrame::filter);

    Ok(PipelineData::Value(
        lazy.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::expressions::ExprCol;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(FilterWith {}), Box::new(ExprCol {})])
//     }
// }
