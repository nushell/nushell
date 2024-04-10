use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, LabeledError, PipelineData, Signature, Value,
};

use crate::{values::PolarsPluginObject, PolarsPlugin};

#[derive(Clone)]
pub struct ListDF;

impl PluginCommand for ListDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars ls"
    }

    fn usage(&self) -> &str {
        "Lists stored dataframes."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a new dataframe and shows it in the dataframe list",
            example: r#"let test = ([[a b];[1 2] [3 4]] | dfr into-df);
    polars ls"#,
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let vals = plugin.cache.process_entries(|(key, value)| match value {
            PolarsPluginObject::NuDataFrame(df) => Ok(Some(Value::record(
                record! {
                    "key" => Value::string(key.to_string(), call.head),
                    "columns" => Value::int(df.as_ref().width() as i64, call.head),
                    "rows" => Value::int(df.as_ref().height() as i64, call.head),
                    "type" => Value::string("NuDataFrame", call.head),
                },
                call.head,
            ))),
            PolarsPluginObject::NuLazyFrame(lf) => {
                let lf = lf.clone().collect(call.head)?;
                Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "columns" => Value::int(lf.as_ref().width() as i64, call.head),
                        "rows" => Value::int(lf.as_ref().height() as i64, call.head),
                        "type" => Value::string("NuLazyFrame", call.head),
                    },
                    call.head,
                )))
            }
            PolarsPluginObject::NuExpression(_) => Ok(Some(Value::record(
                record! {
                    "key" => Value::string(key.to_string(), call.head),
                    "columns" => Value::nothing(call.head),
                    "rows" => Value::nothing(call.head),
                    "type" => Value::string("NuExpression", call.head),
                },
                call.head,
            ))),
            PolarsPluginObject::NuLazyGroupBy(_) => Ok(Some(Value::record(
                record! {
                    "key" => Value::string(key.to_string(), call.head),
                    "columns" => Value::nothing(call.head),
                    "rows" => Value::nothing(call.head),
                    "type" => Value::string("NuLazyGroupBy", call.head),
                },
                call.head,
            ))),
            PolarsPluginObject::NuWhen(_) => Ok(Some(Value::record(
                record! {
                    "key" => Value::string(key.to_string(), call.head),
                    "columns" => Value::nothing(call.head),
                    "rows" => Value::nothing(call.head),
                    "type" => Value::string("NuWhen", call.head),
                },
                call.head,
            ))),
        })?;
        let vals = vals.into_iter().flatten().collect();
        let list = Value::list(vals, call.head);
        Ok(list.into_pipeline_data())
    }
}
