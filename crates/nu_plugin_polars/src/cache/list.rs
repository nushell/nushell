use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, IntoPipelineData, LabeledError, PipelineData, Signature, Value, record,
};

use crate::{PolarsPlugin, values::PolarsPluginObject};

#[derive(Clone)]
pub struct ListDF;

impl PluginCommand for ListDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars store-ls"
    }

    fn description(&self) -> &str {
        "Lists stored polars objects."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Creates a new dataframe and shows it in the dataframe list",
            example: r#"let test = ([[a b];[1 2] [3 4]] | polars into-df);
    polars store-ls"#,
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
        let vals = plugin.cache.process_entries(|(key, value)| {
            let span_contents = engine.get_span_contents(value.span)?;
            let span_contents = String::from_utf8_lossy(&span_contents);
            match &value.value {
                PolarsPluginObject::NuDataFrame(df) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "created" => Value::date(value.created, call.head),
                        "columns" => Value::int(df.as_ref().width() as i64, call.head),
                        "rows" => Value::int(df.as_ref().height() as i64, call.head),
                        "type" => Value::string("DataFrame", call.head),
                        "estimated_size" => Value::filesize(df.to_polars().estimated_size() as i64, call.head),
                        "span_contents" =>  Value::string(span_contents, value.span),
                        "span_start" => Value::int(value.span.start as i64, call.head),
                        "span_end" => Value::int(value.span.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuLazyFrame(lf) => {
                    let lf = lf.clone().collect(call.head)?;
                    Ok(Some(Value::record(
                        record! {
                            "key" => Value::string(key.to_string(), call.head),
                            "created" => Value::date(value.created, call.head),
                            "columns" => Value::int(lf.as_ref().width() as i64, call.head),
                            "rows" => Value::int(lf.as_ref().height() as i64, call.head),
                            "type" => Value::string("LazyFrame", call.head),
                            "estimated_size" => Value::filesize(lf.to_polars().estimated_size() as i64, call.head),
                            "span_contents" =>  Value::string(span_contents, value.span),
                            "span_start" => Value::int(value.span.start as i64, call.head),
                            "span_end" => Value::int(value.span.end as i64, call.head),
                            "reference_count" => Value::int(value.reference_count as i64, call.head),
                        },
                        call.head,
                    )))
                }
                PolarsPluginObject::NuExpression(_) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "created" => Value::date(value.created, call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("Expression", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents, value.span),
                        "span_start" => Value::int(value.span.start as i64, call.head),
                        "span_end" => Value::int(value.span.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuLazyGroupBy(_) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("LazyGroupBy", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents, call.head),
                        "span_start" => Value::int(call.head.start as i64, call.head),
                        "span_end" => Value::int(call.head.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuWhen(_) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("When", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents.to_string(), call.head),
                        "span_start" => Value::int(call.head.start as i64, call.head),
                        "span_end" => Value::int(call.head.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuPolarsTestData(_, _) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("When", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents.to_string(), call.head),
                        "span_start" => Value::int(call.head.start as i64, call.head),
                        "span_end" => Value::int(call.head.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuDataType(_) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "created" => Value::date(value.created, call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("DataType", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents, value.span),
                        "span_start" => Value::int(value.span.start as i64, call.head),
                        "span_end" => Value::int(value.span.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
                PolarsPluginObject::NuSchema(_) => Ok(Some(Value::record(
                    record! {
                        "key" => Value::string(key.to_string(), call.head),
                        "created" => Value::date(value.created, call.head),
                        "columns" => Value::nothing(call.head),
                        "rows" => Value::nothing(call.head),
                        "type" => Value::string("Schema", call.head),
                        "estimated_size" => Value::nothing(call.head),
                        "span_contents" =>  Value::string(span_contents, value.span),
                        "span_start" => Value::int(value.span.start as i64, call.head),
                        "span_end" => Value::int(value.span.end as i64, call.head),
                        "reference_count" => Value::int(value.reference_count as i64, call.head),
                    },
                    call.head,
                ))),
            }
        })?;
        let vals = vals.into_iter().flatten().collect();
        let list = Value::list(vals, call.head);
        Ok(list.into_pipeline_data())
    }
}
