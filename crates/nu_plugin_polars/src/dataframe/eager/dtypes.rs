use crate::{values::CustomValueSupport, Cacheable, PolarsPlugin};

use super::super::values::{Column, NuDataFrame};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct DataTypes;

impl PluginCommand for DataTypes {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars dtypes"
    }

    fn usage(&self) -> &str {
        "Show dataframe data types."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Dataframe dtypes",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars dtypes",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "column".to_string(),
                            vec![Value::test_string("a"), Value::test_string("b")],
                        ),
                        Column::new(
                            "dtype".to_string(),
                            vec![Value::test_string("i64"), Value::test_string("i64")],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let mut dtypes: Vec<Value> = Vec::new();
    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| {
            let dtype = df
                .as_ref()
                .column(v)
                .expect("using name from list of names from dataframe")
                .dtype();

            let dtype_str = dtype.to_string();

            dtypes.push(Value::string(dtype_str, call.head));

            Value::string(*v, call.head)
        })
        .collect();

    let names_col = Column::new("column".to_string(), names);
    let dtypes_col = Column::new("dtype".to_string(), dtypes);

    let df = NuDataFrame::try_from_columns(vec![names_col, dtypes_col], None)?;
    Ok(PipelineData::Value(
        df.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(DataTypes {})])
//     }
// }
