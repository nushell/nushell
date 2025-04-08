use nu_plugin::PluginCommand;
use nu_protocol::{Category, Example, ShellError, Signature, Span, Type, Value};

use crate::{
    values::{CustomValueSupport, NuDataType},
    PolarsPlugin,
};

pub struct ToDataType;

impl PluginCommand for ToDataType {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars to-dtype"
    }

    fn description(&self) -> &str {
        "Convert a string to a specific datatype."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::String, Type::Custom("datatype".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a string to a specific datatype",
            example: r#""i64" | polars to-dtype"#,
            result: Some(Value::string("i64", Span::test_data())),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        command(plugin, engine, call, input).map_err(nu_protocol::LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    input: nu_protocol::PipelineData,
) -> Result<nu_protocol::PipelineData, ShellError> {
    NuDataType::try_from_pipeline(plugin, input, call.head)?
        .to_pipeline_data(plugin, engine, call.head)
}
