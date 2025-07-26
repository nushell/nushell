use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use uuid::Uuid;

use crate::PolarsPlugin;

#[derive(Clone)]
pub struct CacheRemove;

impl PluginCommand for CacheRemove {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars store-rm"
    }

    fn description(&self) -> &str {
        "Removes a stored Dataframe or other object from the plugin cache."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("keys", SyntaxShape::String, "Keys of objects to remove")
            .input_output_type(Type::Any, Type::List(Box::new(Type::String)))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Removes a stored ",
            example: r#"let df = ([[a b];[1 2] [3 4]] | polars into-df);
    polars store-ls | get key | first | polars store-rm $in"#,
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let msgs: Vec<Value> = call
            .rest::<String>(0)?
            .into_iter()
            .map(|ref key| remove_cache_entry(plugin, engine, key, call.head))
            .collect::<Result<Vec<Value>, ShellError>>()?;

        Ok(PipelineData::value(Value::list(msgs, call.head), None))
    }
}

fn remove_cache_entry(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    key: &str,
    span: Span,
) -> Result<Value, ShellError> {
    let key = as_uuid(key, span)?;
    let msg = plugin
        .cache
        .remove(engine, &key, true)?
        .map(|_| format!("Removed: {key}"))
        .unwrap_or_else(|| format!("No value found for key: {key}"));
    Ok(Value::string(msg, span))
}

fn as_uuid(s: &str, span: Span) -> Result<Uuid, ShellError> {
    Uuid::parse_str(s).map_err(|e| ShellError::GenericError {
        error: format!("Failed to convert key string to UUID: {e}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}

#[cfg(test)]
mod test {
    use nu_command::{First, Get};
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::Span;

    use super::*;

    #[test]
    fn test_remove() -> Result<(), ShellError> {
        let plugin = PolarsPlugin::new_test_mode()?.into();
        let pipeline_data = PluginTest::new("polars", plugin)?
            .add_decl(Box::new(First))?
            .add_decl(Box::new(Get))?
            .eval("let df = ([[a b];[1 2] [3 4]] | polars into-df); polars store-ls | get key | first | polars store-rm $in")?;
        let value = pipeline_data.into_value(Span::test_data())?;
        let msg = value
            .as_list()?
            .first()
            .expect("there should be a first entry")
            .as_str()?;
        assert!(msg.contains("Removed"));
        Ok(())
    }
}
