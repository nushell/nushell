use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use crate::{
    PolarsPlugin,
    dataframe::{utils::extract_strings, values::NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType},
};

use crate::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct RenameDF;

impl PluginCommand for RenameDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars rename"
    }

    fn description(&self) -> &str {
        "Rename a dataframe column."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "columns",
                SyntaxShape::Any,
                "Column(s) to be renamed. A string or list of strings",
            )
            .required(
                "new names",
                SyntaxShape::Any,
                "New names for the selected column(s). A string or list of strings",
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
                description: "Renames a series",
                example: "[5 6 7 8] | polars into-df | polars rename '0' new_name",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "new_name".to_string(),
                            vec![
                                Value::test_int(5),
                                Value::test_int(6),
                                Value::test_int(7),
                                Value::test_int(8),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Renames a dataframe column",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars rename a a_new",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a_new".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Renames two dataframe columns",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars rename [a b] [a_new b_new]",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a_new".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b_new".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value).map_err(LabeledError::from)? {
            PolarsPluginObject::NuDataFrame(df) => {
                command_eager(plugin, engine, call, df).map_err(LabeledError::from)
            }
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_lazy(plugin, engine, call, lazy).map_err(LabeledError::from)
            }
            _ => Err(LabeledError::new(format!("Unsupported type: {value:?}"))
                .with_label("Unsupported Type", call.head)),
        }
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let columns: Value = call.req(0)?;
    let columns = extract_strings(columns)?;

    let new_names: Value = call.req(1)?;
    let new_names = extract_strings(new_names)?;

    let mut polars_df = df.to_polars();

    for (from, to) in columns.iter().zip(new_names.iter()) {
        polars_df
            .rename(from, to.into())
            .map_err(|e| ShellError::GenericError {
                error: "Error renaming".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;
    }

    let df = NuDataFrame::new(false, polars_df);
    df.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let columns: Value = call.req(0)?;
    let columns = extract_strings(columns)?;

    let new_names: Value = call.req(1)?;
    let new_names = extract_strings(new_names)?;

    if columns.len() != new_names.len() {
        let value: Value = call.req(1)?;
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "New name list has different size to column list".into(),
            span: value.span(),
        });
    }

    let lazy = lazy.to_polars();
    let lazy: NuLazyFrame = lazy.rename(&columns, &new_names, true).into();

    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&RenameDF)
    }
}
