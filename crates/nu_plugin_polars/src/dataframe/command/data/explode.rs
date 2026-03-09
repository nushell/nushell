use std::sync::Arc;

use crate::PolarsPlugin;
use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use crate::values::{CustomValueSupport, NuSelector, PolarsPluginObject, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::{ExplodeOptions, PlSmallStr, Selector};

#[derive(Clone)]
pub struct LazyExplode;

impl PluginCommand for LazyExplode {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars explode"
    }

    fn description(&self) -> &str {
        "Explodes a dataframe or creates a explode expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("empty-as-null", "Explode an empty list into a `null`", None)
            .switch("keep-nulls", "Explode a `null` into a `null`", None)
            .rest(
                "columns",
                SyntaxShape::Any,
                "A polars selector or columns to explode, only applicable for dataframes",
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Explode the specified dataframe",
                example:
                    "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] 
                    | polars into-df 
                    | polars explode hobbies 
                    | polars collect
                    | polars sort-by [id, name]",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "id".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(2),
                                ],
                            ),
                            Column::new(
                                "name".to_string(),
                                vec![
                                    Value::test_string("Mercy"),
                                    Value::test_string("Mercy"),
                                    Value::test_string("Bob"),
                                    Value::test_string("Bob"),
                                ],
                            ),
                            Column::new(
                                "hobbies".to_string(),
                                vec![
                                    Value::test_string("Cycling"),
                                    Value::test_string("Knitting"),
                                    Value::test_string("Skiing"),
                                    Value::test_string("Football"),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Select a column and explode the values",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | polars into-df | polars select (polars col hobbies | polars explode)",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "hobbies".to_string(),
                            vec![
                                Value::test_string("Cycling"),
                                Value::test_string("Knitting"),
                                Value::test_string("Skiing"),
                                Value::test_string("Football"),
                            ],
                        )],
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
        explode(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

pub(crate) fn explode(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head)?;
    let explode_options = ExplodeOptions {
        empty_as_null: call.has_flag("empty-as-null")?,
        keep_nulls: call.has_flag("keep-nulls")?,
    };
    match PolarsPluginObject::try_from_value(plugin, &value)? {
        PolarsPluginObject::NuDataFrame(df) => {
            let lazy = df.lazy();
            explode_lazy(plugin, engine, call, lazy, explode_options)
        }
        PolarsPluginObject::NuLazyFrame(lazy) => {
            explode_lazy(plugin, engine, call, lazy, explode_options)
        }
        PolarsPluginObject::NuExpression(expr) => {
            explode_expr(plugin, engine, call, expr, explode_options)
        }
        _ => Err(ShellError::CantConvert {
            to_type: "dataframe or expression".into(),
            from_type: value.get_type().to_string(),
            span: call.head,
            help: None,
        }),
    }
}

pub(crate) fn explode_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    explode_options: ExplodeOptions,
) -> Result<PipelineData, ShellError> {
    let selector = if let Some(Ok(selector)) = call
        .positional
        .first()
        .map(|v| NuSelector::try_from_value(plugin, v))
    {
        selector.into_polars()
    } else {
        let columns = call
            .positional
            .iter()
            .map(|param| param.as_str().map(|s| s.to_string()))
            .map(|s_result| s_result.map(|ref s| PlSmallStr::from_str(s)))
            .collect::<Result<Vec<PlSmallStr>, ShellError>>()?;

        let columns = Arc::from(columns);

        Selector::ByName {
            names: columns,
            strict: true,
        }
    };

    let exploded = lazy.to_polars().explode(selector, explode_options);
    let lazy = NuLazyFrame::from(exploded);
    lazy.to_pipeline_data(plugin, engine, call.head)
}

pub(crate) fn explode_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
    explode_options: ExplodeOptions,
) -> Result<PipelineData, ShellError> {
    let expr: NuExpression = expr.into_polars().explode(explode_options).into();
    expr.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyExplode)
    }
}
