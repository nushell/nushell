use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars_lazy::dsl::col;

use crate::values::{CustomValueSupport, NuDataFrame};
use crate::PolarsPlugin;

use super::super::values::utils::convert_columns_string;
use super::super::values::{Column, NuLazyFrame};

#[derive(Clone)]
pub struct DropNulls;

impl PluginCommand for DropNulls {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars drop-nulls"
    }

    fn usage(&self) -> &str {
        "Drops null values in dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "subset",
                SyntaxShape::Table(vec![]),
                "subset of columns to drop nulls",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "drop null values in dataframe",
                example: r#"let df = ([[a b]; [1 2] [3 0] [1 2]] | polars into-df);
    let a = ($df | polars with-column [((polars col b) / (polars col b) | polars as res)]);
    $a | polars drop-nulls"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(1)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(2)],
                            ),
                            Column::new(
                                "res".to_string(),
                                vec![Value::test_int(1), Value::test_int(1)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "drop null values in dataframe",
                example: r#"let s = ([1 2 0 0 3 4] | polars into-df);
    ($s / $s) | polars drop-nulls"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "div_0_0".to_string(),
                            vec![
                                Value::test_int(1),
                                Value::test_int(1),
                                Value::test_int(1),
                                Value::test_int(1),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let columns: Option<Vec<Value>> = call.opt(0)?;

    let (subset, _col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns_string(cols, call.head)?;
            let agg_expr = agg_string.iter().map(|s| col(s)).collect();
            (Some(agg_expr), col_span)
        }
        None => (None, call.head),
    };

    let polars_df = df.to_polars().drop_nulls(subset);
    let df = NuLazyFrame::new(polars_df);
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&DropNulls)
    }
}
