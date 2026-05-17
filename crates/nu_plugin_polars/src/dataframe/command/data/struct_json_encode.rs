use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type};
use polars::df;

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame, NuExpression},
};

#[derive(Clone)]
pub struct StructJsonEncode;

impl PluginCommand for StructJsonEncode {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars struct-json-encode"
    }

    fn description(&self) -> &str {
        "Convert this struct to a string column with json values."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Custom("dataframe".into()))
            .input_output_type(Type::custom("expression"), Type::custom("expression"))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Encode a struct as JSON",
            example: r#"[[id person]; [1 {name: "Bob", age: 36}] [2 {name: "Betty", age: 63}]] 
                    | polars into-df -s {id: i32, person: {name: str, age: u8}} 
                    | polars select id (polars col person | polars struct-json-encode | polars as encoded)
                    | polars sort-by id
                    | polars collect"#,
            result: Some(
                NuDataFrame::from(
                    df!(
                        "id" => [1i32, 2],
                        "encoded" => [
                            r#"{"name":"Bob","age":36}"#,
                            r#"{"name":"Betty","age":63}"#,
                        ],
                    )
                    .expect("Should be able to create a simple dataframe"),
                )
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
        NuExpression::try_from_pipeline(plugin, input, call.head)
            .map(|expr| expr.into_polars().struct_().json_encode())
            .map(NuExpression::from)
            .and_then(|expr| expr.to_pipeline_data(plugin, engine, call.head))
            .map_err(LabeledError::from)
    }
}
