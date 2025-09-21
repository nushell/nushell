use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::{prelude::NamedFrom, series::Series};
use uuid::Uuid;

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame},
};

#[derive(Clone)]
pub struct CacheGet;

impl PluginCommand for CacheGet {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars store-get"
    }

    fn description(&self) -> &str {
        "Gets a Dataframe or other object from the plugin cache."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("key", SyntaxShape::String, "Key of objects to get")
            .input_output_types(vec![
                (Type::Any, Type::Custom("dataframe".into())),
                (Type::Any, Type::Custom("expression".into())),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get a stored object",
            example: r#"let df = ([[a b];[1 2] [3 4]] | polars into-df);
    polars store-ls | get key | first | polars store-get $in"#,
            result: Some(
                NuDataFrame::try_from_series_vec(
                    vec![
                        Series::new("a".into(), &[1_i64, 3]),
                        Series::new("b".into(), &[2_i64, 4]),
                    ],
                    Span::test_data(),
                )
                .expect("could not create dataframe")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let key = call
            .req::<String>(0)
            .and_then(|ref k| as_uuid(k, call.head))?;

        let value = if let Some(cache_value) = plugin.cache.get(&key, true)? {
            let polars_object = cache_value.value;
            polars_object.into_value(call.head)
        } else {
            Value::nothing(call.head)
        };

        Ok(PipelineData::value(value, None))
    }
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
    use super::*;
    use crate::test::test_polars_plugin_command_with_decls;
    use nu_command::{First, Get};

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(&CacheGet, vec![Box::new(Get), Box::new(First)])
    }
}
