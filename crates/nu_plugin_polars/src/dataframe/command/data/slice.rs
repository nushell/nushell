use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use crate::{PolarsPlugin, dataframe::values::Column, values::CustomValueSupport};

use crate::values::{NuDataFrame, NuLazyFrame, PolarsPluginObject, PolarsPluginType};

#[derive(Clone)]
pub struct SliceDF;

impl PluginCommand for SliceDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars slice"
    }

    fn description(&self) -> &str {
        "Creates new dataframe from a slice of rows."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("offset", SyntaxShape::Int, "Start of slice.")
            .required("size", SyntaxShape::Int, "Size of slice.")
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
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create new dataframe from a slice of the rows",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars slice 0 1",
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
                description: "Create a new lazy dataframe from a slice of a lazy dataframe's rows",
                example: "[[a b]; [1 2] [3 4]] | polars into-lazy | polars slice 0 1 | describe",
                result: Some(Value::test_string("polars_lazyframe")),
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
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_lazy(plugin, engine, call, lazy).map_err(|e| e.into())
            }
            _ => {
                let df = NuDataFrame::try_from_value_coerce(plugin, &value, call.head)?;
                command_eager(plugin, engine, call, df).map_err(LabeledError::from)
            }
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
    let offset: i64 = call.req(0)?;
    let size: usize = call.req(1)?;

    let res = df.as_ref().slice(offset, size);
    let res = NuDataFrame::new(false, res);

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let offset: i64 = call.req(0)?;
    let size: u64 = call.req(1)?;

    let res: NuLazyFrame = lazy.to_polars().slice(offset, size).into(); //.limit(rows).into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&SliceDF)
    }
}
