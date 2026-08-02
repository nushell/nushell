use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuDataFrame, NuDataType, PolarsPluginObject, PolarsPluginType,
        cant_convert_err,
    },
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value, engine::Closure,
};
use polars::{
    df,
    prelude::{DataType, Series},
};

#[derive(Clone)]
pub struct MapBatches;

impl PluginCommand for MapBatches {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars map-batches"
    }

    fn description(&self) -> &str {
        "Map a custom Nushell closure over one or more dataframe columns."
    }

    fn extra_description(&self) -> &str {
        "The closure receives a list of single-column dataframes (one per named column) \
        and must return a value that can be converted to a series — a single-column \
        dataframe, a list, or a scalar. The result is returned as a single-column \
        dataframe. The closure is invoked once with all columns at the time \
        `polars map-batches` is run."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Closure to apply. Receives a list of single-column dataframes.",
            )
            .rest(
                "columns",
                SyntaxShape::String,
                "Names of columns to pass to the closure. If omitted, all columns are used.",
            )
            .named(
                "return-dtype",
                SyntaxShape::Any,
                "Data type to cast the closure's result to.",
                Some('d'),
            )
            .named(
                "name",
                SyntaxShape::String,
                "Name for the resulting column.",
                Some('n'),
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return a constant series from a closure",
                example: r#"[[a b]; [1 4] [2 5] [3 6]]
    | polars into-df
    | polars map-batches --name out { |_cols| [10 20 30] } a"#,
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "out" => [10, 20, 30]
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Double the values of column `a` via a Nushell closure",
                example: r#"[[a b]; [1 4] [2 5] [3 6]]
    | polars into-df
    | polars map-batches { |cols| $cols | first | polars get a | each { |v| $v * 2 } } a"#,
                result: None,
            },
            Example {
                description: "Sum two columns element-wise and rename the result",
                example: r#"[[a b]; [1 4] [2 5] [3 6]]
    | polars into-df
    | polars map-batches --name a_plus_b { |cols|
        let a = $cols | get 0 | polars get a
        let b = $cols | get 1 | polars get b
        $a | zip $b | each { |pair| $pair.0 + $pair.1 }
      } a b"#,
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "a_plus_b" => [5, 7, 9]
                        )
                        .expect("simple df for test should not fail"),
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
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let value = input.into_value(call.head)?;

        let df = match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => df,
            PolarsPluginObject::NuLazyFrame(lazy) => lazy.collect(call.head)?,
            _ => {
                return Err(cant_convert_err(
                    &value,
                    &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
                )
                .into());
            }
        };

        let closure: Spanned<Closure> = call.req(0)?;
        let columns: Vec<Spanned<String>> = call.rest(1)?;
        let return_dtype: Option<Value> = call.get_flag("return-dtype")?;
        let name_flag: Option<Spanned<String>> = call.get_flag("name")?;

        let series_list = collect_series(&df, &columns)?;
        let inputs = series_to_value_list(plugin, engine, series_list, call.head)?;

        let result_value = engine
            .eval_closure(&closure, vec![inputs], None)
            .inspect_err(|e| eprintln!("Error evaluating closure in polars map-batches: {e}"))
            .map_err(LabeledError::from)?;

        let mut result_series = value_to_series(plugin, result_value, call.head)?;

        if let Some(dtype_value) = return_dtype {
            let dtype = NuDataType::try_from_value(plugin, &dtype_value)?.to_polars();
            result_series = cast_series(result_series, &dtype, call.head)?;
        }

        if let Some(name) = name_flag {
            result_series.rename(name.item.into());
        }

        let out_df = NuDataFrame::try_from_series(result_series, call.head)?;
        out_df
            .to_pipeline_data(plugin, engine, call.head)
            .map(|pd| pd.set_metadata(metadata))
            .map_err(LabeledError::from)
    }
}

fn collect_series(
    df: &NuDataFrame,
    columns: &[Spanned<String>],
) -> Result<Vec<Series>, ShellError> {
    let polars_df = df.to_polars();
    if columns.is_empty() {
        Ok(polars_df
            .columns()
            .iter()
            .map(|c| c.as_materialized_series().clone())
            .collect())
    } else {
        columns
            .iter()
            .map(|name| {
                polars_df
                    .column(name.item.as_str())
                    .map(|c| c.as_materialized_series().clone())
                    .map_err(|e| {
                        ShellError::Generic(GenericError::new(
                            format!("Column '{}' not found: {e}", name.item),
                            "",
                            name.span,
                        ))
                    })
            })
            .collect()
    }
}

fn series_to_value_list(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    series_list: Vec<Series>,
    span: Span,
) -> Result<Value, ShellError> {
    let values: Vec<Value> = series_list
        .into_iter()
        .map(|s| {
            let df = NuDataFrame::try_from_series(s, span)?;
            df.cache_and_to_value(plugin, engine, span)
        })
        .collect::<Result<_, ShellError>>()?;
    Ok(Value::list(values, span))
}

fn value_to_series(plugin: &PolarsPlugin, value: Value, span: Span) -> Result<Series, ShellError> {
    match &value {
        Value::Custom { .. } => {
            let obj = PolarsPluginObject::try_from_value(plugin, &value)?;
            match obj {
                PolarsPluginObject::NuDataFrame(df) => df.as_series(span),
                PolarsPluginObject::NuLazyFrame(lazy) => lazy.collect(span)?.as_series(span),
                _ => Err(ShellError::CantConvert {
                    to_type: "series".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: Some(
                        "closure must return a single-column dataframe, a list, or a scalar".into(),
                    ),
                }),
            }
        }
        Value::List { vals, .. } => {
            let df = NuDataFrame::try_from_iter(plugin, vals.iter().cloned(), None, span)?;
            df.as_series(span)
        }
        _ => {
            let single = Value::list(vec![value.clone()], span);
            let df = NuDataFrame::try_from_iter(plugin, std::iter::once(single), None, span)?;
            df.as_series(span)
        }
    }
}

fn cast_series(series: Series, dtype: &DataType, span: Span) -> Result<Series, ShellError> {
    series.cast(dtype).map_err(|e| {
        ShellError::Generic(GenericError::new(
            format!("Error casting result to {dtype:?}: {e}"),
            "",
            span,
        ))
    })
}

#[cfg(test)]
mod test {
    use nu_command::{Each, Get, Zip};

    use super::*;
    use crate::test::test_polars_plugin_command_with_decls;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(
            &MapBatches,
            vec![Box::new(Zip), Box::new(Get), Box::new(Each)],
        )
    }
}
