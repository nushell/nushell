use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginObject, datatype_list},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value, record,
};

#[derive(Clone)]
pub struct SchemaCmd;

impl PluginCommand for SchemaCmd {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars schema"
    }

    fn description(&self) -> &str {
        "Show schema for a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("datatype-list", "creates a lazy dataframe", Some('l'))
            .input_output_type(Type::Any, Type::record())
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Dataframe schema",
            example: r#"[[a b]; [1 "foo"] [3 "bar"]] | polars into-df | polars schema"#,
            result: Some(Value::record(
                record! {
                    "a" => Value::string("i64", Span::test_data()),
                    "b" => Value::string("str", Span::test_data()),
                },
                Span::test_data(),
            )),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        if call.has_flag("datatype-list")? {
            Ok(PipelineData::value(datatype_list(Span::unknown()), None))
        } else {
            command(plugin, engine, call, input).map_err(LabeledError::from)
        }
    }
}

fn command(
    plugin: &PolarsPlugin,
    _engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match PolarsPluginObject::try_from_pipeline(plugin, input, call.head)? {
        PolarsPluginObject::NuDataFrame(df) => {
            let schema = df.schema();
            let value = schema.base_value(call.head)?;
            Ok(PipelineData::value(value, None))
        }
        PolarsPluginObject::NuLazyFrame(mut lazy) => {
            let schema = lazy.schema()?;
            let value = schema.base_value(call.head)?;
            Ok(PipelineData::value(value, None))
        }
        _ => Err(ShellError::GenericError {
            error: "Must be a dataframe or lazy dataframe".into(),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&SchemaCmd)
    }
}
