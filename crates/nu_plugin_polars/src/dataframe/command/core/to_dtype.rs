use nu_plugin::PluginCommand;
use nu_protocol::{Category, Example, ShellError, Signature, Span, Type, Value};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataType, PolarsPluginType},
};

pub struct ToDataType;

impl PluginCommand for ToDataType {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-dtype"
    }

    fn description(&self) -> &str {
        "Convert a string to a specific datatype."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::String, PolarsPluginType::NuDataFrame.into())
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Convert a string to a specific datatype and back to a nu object",
            example: r#"'i64' | polars into-dtype | polars into-nu"#,
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

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;
    use nu_protocol::ShellError;

    #[test]
    fn test_into_dtype() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToDataType)
    }
}
