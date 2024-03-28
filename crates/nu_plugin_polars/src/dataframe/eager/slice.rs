use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::{dataframe::values::Column, Cacheable, CustomValueSupport, PolarsPlugin};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct SliceDF;

impl PluginCommand for SliceDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars slice"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe from a slice of rows."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("offset", SyntaxShape::Int, "start of slice")
            .required("size", SyntaxShape::Int, "size of slice")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
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
    let offset: i64 = call.req(0)?;
    let size: usize = call.req(1)?;

    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let res = df.as_ref().slice(offset, size);
    let res = NuDataFrame::new(false, res);

    Ok(PipelineData::Value(
        res.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(SliceDF {})])
//     }
// }
