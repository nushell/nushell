use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{
    record, Category, IntoPipelineData, PipelineData, PluginExample, PluginSignature, Value,
};

use crate::{CacheValue, DataFrameCache, PolarsDataFramePlugin};

#[derive(Clone)]
pub struct ListDF;

impl PluginCommand for ListDF {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars ls")
            .usage("Lists stored dataframes.")
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(vec![PluginExample {
                description: "Creates a new dataframe and shows it in the dataframe list".into(),
                example: r#"let test = ([[a b];[1 2] [3 4]] | dfr into-df);
    ls"#
                .into(),
                result: None,
            }])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let vals = DataFrameCache::process_entries(|(key, value)| match value {
            CacheValue::DataFrame(df) => Ok(Some(Value::record(
                record! {
                    "key" => Value::string(key.to_string(), call.head),
                    "columns" => Value::int(df.as_ref().width() as i64, call.head),
                    "rows" => Value::int(df.as_ref().height() as i64, call.head),
                },
                call.head,
            ))),
            CacheValue::LazyFrame(lf) => {
                let lf = lf.clone().collect(call.head)?;
                Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "columns" => Value::int(lf.as_ref().width() as i64, call.head),
                        "rows" => Value::int(lf.as_ref().height() as i64, call.head),
                    },
                    call.head,
                )))
            }
            _ => Ok(None),
        })?;
        let vals = vals.into_iter().flatten().collect();
        let list = Value::list(vals, call.head);
        Ok(list.into_pipeline_data())
    }
}
