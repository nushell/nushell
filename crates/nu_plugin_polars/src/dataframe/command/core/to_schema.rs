use nu_plugin::PluginCommand;
use nu_protocol::{Category, Example, ShellError, Signature, Span, Type, Value, record};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuSchema, PolarsPluginType},
};

pub struct ToSchema;

impl PluginCommand for ToSchema {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-schema"
    }

    fn description(&self) -> &str {
        "Convert a value to a polars schema object"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, PolarsPluginType::NuSchema.into())
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Convert a record into a schema and back to a nu object",
            example: r#"{a: str, b: u8} | polars into-schema | polars into-nu"#,
            result: Some(Value::record(
                record! {
                    "a" => Value::string("str", Span::test_data()),
                    "b" => Value::string("u8", Span::test_data()),
                },
                Span::test_data(),
            )),
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
    NuSchema::try_from_pipeline(plugin, input, call.head)?
        .to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;
    use nu_protocol::ShellError;

    #[test]
    fn test_into_schema() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToSchema)
    }
}
