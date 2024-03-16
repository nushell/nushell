use crate::PolarsDataFramePlugin;

use super::super::values::NuDataFrame;
use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{
    Category, PipelineData, PluginExample, PluginSignature, ShellError, Type, Value,
};

#[derive(Clone)]
pub struct ColumnsDF;

impl PluginCommand for ColumnsDF {
    type Plugin = PolarsDataFramePlugin;
    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars columns")
            .usage("Show dataframe columns.")
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(call, input).map_err(|e| e.into())
    }
}

fn examples() -> Vec<PluginExample> {
    vec![PluginExample {
        description: "Dataframe columns".into(),
        example: "[[a b]; [1 2] [3 4]] | polars into-df | polars columns".into(),
        //     result: Some(Value::list(
        //         vec![Value::test_string("a"), Value::test_string("b")],
        //         Span::test_data(),
        //     )),
        result: None,
    }]
}
fn command(call: &EvaluatedCall, input: PipelineData) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| Value::string(*v, call.head))
        .collect();

    let names = Value::list(names, call.head);

    Ok(PipelineData::Value(names, None))
}

// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(ColumnsDF {})])
//     }
// }
