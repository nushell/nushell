use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::LazyFrame;

use crate::{
    PolarsPlugin,
    dataframe::values::{NuExpression, NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use crate::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct FilterWith;

impl PluginCommand for FilterWith {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars filter-with"
    }

    fn description(&self) -> &str {
        "Filters dataframe using a mask or expression as reference."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "mask or expression",
                SyntaxShape::Any,
                "boolean mask used to filter data",
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                    .into_value(Span::test_data()),
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
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

        lazy.to_pipeline_data(plugin, engine, call.head)
    } else {
        let mask = NuDataFrame::try_from_value_coerce(plugin, &mask_value, mask_span)?
            .as_series(mask_span)?;
        let mask = mask.bool().map_err(|e| ShellError::GenericError {
            error: "Error casting to bool".into(),
            msg: e.to_string(),
            span: Some(mask_span),
            help: Some("Perhaps you want to use a series with booleans as mask".into()),
            inner: vec![],
        })?;

        let polars_df = df
            .as_ref()
            .filter(mask)
            .map_err(|e| ShellError::GenericError {
                error: "Error filtering dataframe".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: Some("The only allowed column types for dummies are String or Int".into()),
                inner: vec![],
            })?;
        let df = NuDataFrame::new(df.from_lazy, polars_df);
        df.to_pipeline_data(plugin, engine, call.head)
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
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&FilterWith)
    }
}
