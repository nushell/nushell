use nu_plugin::PluginCommand;
use nu_protocol::{record, Category, Example, ShellError, Signature, Span, Type, Value};

use crate::{
    values::{CustomValueSupport, NuSchema},
    PolarsPlugin,
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
            .input_output_type(Type::Any, Type::Custom("schema".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a record into a schema",
            example: r#"{a: str, b: u8} | polars into-schema"#,
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
