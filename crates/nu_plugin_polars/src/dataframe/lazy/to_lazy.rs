use crate::{dataframe::values::NuSchema, values::CustomValueSupport, Cacheable, PolarsPlugin};

use super::super::values::{NuDataFrame, NuLazyFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct ToLazyFrame;

impl PluginCommand for ToLazyFrame {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-lazy"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe into a lazy dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "schema",
                SyntaxShape::Record(vec![]),
                r#"Polars Schema in format [{name: str}]. CSV, JSON, and JSONL files"#,
                Some('s'),
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a dictionary and creates a lazy dataframe",
            example: "[[a b];[1 2] [3 4]] | polars into-lazy",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let maybe_schema = call
            .get_flag("schema")?
            .map(|schema| NuSchema::try_from(&schema))
            .transpose()?;

        let df = NuDataFrame::try_from_iter(plugin, input.into_iter(), maybe_schema)?;
        let lazy = NuLazyFrame::from_dataframe(df);
        // We don't want this converted back to an eager dataframe at some point
        Ok(PipelineData::Value(
            lazy.cache(plugin, engine, call.head)?.into_value(call.head),
            None,
        ))
    }
}
