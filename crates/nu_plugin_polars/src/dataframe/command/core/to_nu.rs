use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value, record,
};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use crate::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl PluginCommand for ToNu {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-nu"
    }

    fn description(&self) -> &str {
        "Converts a dataframe or an expression into nushell value for access and exploration."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "rows",
                SyntaxShape::Number,
                "number of rows to be shown",
                Some('n'),
            )
            .switch("tail", "shows tail rows", Some('t'))
            .switch("index", "add an index column", Some('i'))
            .input_output_types(vec![
                (Type::Custom("expression".into()), Type::Any),
                (Type::Custom("dataframe".into()), Type::table()),
                (Type::Custom("datatype".into()), Type::Any),
                (Type::Custom("schema".into()), Type::Any),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        let rec_1 = Value::test_record(record! {
            "index" => Value::test_int(0),
            "a" =>     Value::test_int(1),
            "b" =>     Value::test_int(2),
        });
        let rec_2 = Value::test_record(record! {
            "index" => Value::test_int(1),
            "a" =>     Value::test_int(3),
            "b" =>     Value::test_int(4),
        });
        let rec_3 = Value::test_record(record! {
            "index" => Value::test_int(2),
            "a" =>     Value::test_int(3),
            "b" =>     Value::test_int(4),
        });

        vec![
            Example {
                description: "Shows head rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars into-nu --index",
                result: Some(Value::list(vec![rec_1, rec_2], Span::test_data())),
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "[[a b]; [1 2] [5 6] [3 4]] | polars into-df | polars into-nu --tail --rows 1 --index",
                result: Some(Value::list(vec![rec_3], Span::test_data())),
            },
            Example {
                description: "Convert a col expression into a nushell value",
                example: "polars col a | polars into-nu --index",
                result: Some(Value::test_record(record! {
                    "expr" =>  Value::test_string("column"),
                    "value" => Value::test_string("a"),
                })),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    _engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head)?;
    match PolarsPluginObject::try_from_value(plugin, &value)? {
        PolarsPluginObject::NuDataFrame(df) => dataframe_command(call, df),
        PolarsPluginObject::NuLazyFrame(lazy) => dataframe_command(call, lazy.collect(call.head)?),
        PolarsPluginObject::NuExpression(expr) => {
            let value = expr.to_value(call.head)?;
            Ok(PipelineData::value(value, None))
        }
        PolarsPluginObject::NuDataType(dt) => {
            let value = dt.base_value(call.head)?;
            Ok(PipelineData::value(value, None))
        }
        PolarsPluginObject::NuSchema(schema) => {
            let value = schema.base_value(call.head)?;
            Ok(PipelineData::value(value, None))
        }
        _ => Err(cant_convert_err(
            &value,
            &[
                PolarsPluginType::NuDataFrame,
                PolarsPluginType::NuLazyFrame,
                PolarsPluginType::NuExpression,
                PolarsPluginType::NuDataType,
                PolarsPluginType::NuSchema,
            ],
        )),
    }
}

fn dataframe_command(call: &EvaluatedCall, df: NuDataFrame) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.get_flag("rows")?;
    let tail: bool = call.has_flag("tail")?;
    let index: bool = call.has_flag("index")?;

    let values = if tail {
        df.tail(rows, index, call.head)?
    } else {
        // if rows is specified, return those rows, otherwise return everything
        if rows.is_some() {
            df.head(rows, index, call.head)?
        } else {
            df.head(Some(df.height()), index, call.head)?
        }
    };

    let value = Value::list(values, call.head);

    Ok(PipelineData::value(value, None))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToNu)
    }
}
