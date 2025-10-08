use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{Column, NuDataFrame, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::{
    chunked_array::ChunkedArray,
    prelude::{
        AnyValue, Column as PolarsColumn, DataFrame, DataType, Float64Type, IntoSeries,
        NewChunkedArray, QuantileMethod, StringType,
    },
};

#[derive(Clone)]
pub struct Summary;

impl PluginCommand for Summary {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars summary"
    }

    fn description(&self) -> &str {
        "For a dataframe, produces descriptive statistics (summary statistics) for its numeric columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Custom("dataframe".into()))
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
            .named(
                "quantiles",
                SyntaxShape::List(Box::new(SyntaxShape::Float)),
                "provide optional quantiles",
                Some('q'),
            )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "list dataframe descriptives",
            example: "[[a b]; [1 1] [1 1]] | polars into-df | polars summary",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "descriptor".to_string(),
                            vec![
                                Value::test_string("count"),
                                Value::test_string("sum"),
                                Value::test_string("mean"),
                                Value::test_string("median"),
                                Value::test_string("std"),
                                Value::test_string("min"),
                                Value::test_string("25%"),
                                Value::test_string("50%"),
                                Value::test_string("75%"),
                                Value::test_string("max"),
                            ],
                        ),
                        Column::new(
                            "a (i64)".to_string(),
                            vec![
                                Value::test_float(2.0),
                                Value::test_float(2.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(0.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                            ],
                        ),
                        Column::new(
                            "b (i64)".to_string(),
                            vec![
                                Value::test_float(2.0),
                                Value::test_float(2.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(0.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                            ],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let quantiles: Option<Vec<Value>> = call.get_flag("quantiles")?;
    let quantiles = quantiles.map(|values| {
        values
            .iter()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Float { val, .. } => {
                        if (&0.0..=&1.0).contains(&val) {
                            Ok(*val)
                        } else {
                            Err(ShellError::GenericError {
                                error: "Incorrect value for quantile".into(),
                                msg: "value should be between 0 and 1".into(),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            })
                        }
                    }
                    Value::Error { error, .. } => Err(*error.clone()),
                    _ => Err(ShellError::GenericError {
                        error: "Incorrect value for quantile".into(),
                        msg: "value should be a float".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                }
            })
            .collect::<Result<Vec<f64>, ShellError>>()
    });

    let quantiles = match quantiles {
        Some(quantiles) => quantiles?,
        None => vec![0.25, 0.50, 0.75],
    };

    let mut quantiles_labels = quantiles
        .iter()
        .map(|q| Some(format!("{}%", q * 100.0)))
        .collect::<Vec<Option<String>>>();
    let mut labels = vec![
        Some("count".to_string()),
        Some("sum".to_string()),
        Some("mean".to_string()),
        Some("median".to_string()),
        Some("std".to_string()),
        Some("min".to_string()),
    ];
    labels.append(&mut quantiles_labels);
    labels.push(Some("max".to_string()));

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let names =
        ChunkedArray::<StringType>::from_slice_options("descriptor".into(), &labels).into_series();

    let head = std::iter::once(names);

    let tail = df
        .as_ref()
        .iter()
        .filter(|col| !matches!(col.dtype(), &DataType::Object("object")))
        .map(|col| {
            let count = col.len() as f64;

            let sum = col.sum::<f64>().ok();
            let mean = col.mean();
            let median = col.median();
            let std = col.std(0);
            let min = col.min::<f64>().ok().flatten();

            let mut quantiles = quantiles
                .clone()
                .into_iter()
                .map(|q| {
                    col.quantile_reduce(q, QuantileMethod::default())
                        .ok()
                        .map(|s| s.into_series("quantile".into()))
                        .and_then(|ca| ca.cast(&DataType::Float64).ok())
                        .and_then(|ca| match ca.get(0) {
                            Ok(AnyValue::Float64(v)) => Some(v),
                            _ => None,
                        })
                })
                .collect::<Vec<Option<f64>>>();

            let max = col.max::<f64>().ok().flatten();

            let mut descriptors = vec![Some(count), sum, mean, median, std, min];
            descriptors.append(&mut quantiles);
            descriptors.push(max);

            let name = format!("{} ({})", col.name(), col.dtype());
            ChunkedArray::<Float64Type>::from_slice_options(name.into(), &descriptors).into_series()
        });

    let res = head
        .chain(tail)
        .map(PolarsColumn::from)
        .collect::<Vec<PolarsColumn>>();

    let polars_df = DataFrame::new(res).map_err(|e| ShellError::GenericError {
        error: "Dataframe Error".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let df = NuDataFrame::new(df.from_lazy, polars_df);

    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Summary)
    }
}
