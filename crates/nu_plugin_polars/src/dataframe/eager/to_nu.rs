use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, Example, LabeledError, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

use crate::{
    dataframe::values::NuExpression,
    values::{CustomValueSupport, NuLazyFrame},
    PolarsPlugin,
};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl PluginCommand for ToNu {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-nu"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe or an expression into into nushell value for access and exploration."
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
            .input_output_types(vec![
                (Type::Custom("expression".into()), Type::Any),
                (Type::Custom("dataframe".into()), Type::table()),
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
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars into-nu",
                result: Some(Value::list(vec![rec_1, rec_2], Span::test_data())),
            },
            Example {
                description: "Shows tail rows from dataframe",
                example:
                    "[[a b]; [1 2] [5 6] [3 4]] | polars into-df | polars into-nu --tail --rows 1",
                result: Some(Value::list(vec![rec_3], Span::test_data())),
            },
            Example {
                description: "Convert a col expression into a nushell value",
                example: "polars col a | polars into-nu",
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
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) || NuLazyFrame::can_downcast(&value) {
            dataframe_command(plugin, call, value)
        } else {
            expression_command(plugin, call, value)
        }
        .map_err(|e| e.into())
    }
}

fn dataframe_command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: Value,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.get_flag("rows")?;
    let tail: bool = call.has_flag("tail")?;

    let df = NuDataFrame::try_from_value_coerce(plugin, &input, call.head)?;

    let values = if tail {
        df.tail(rows, call.head)?
    } else {
        // if rows is specified, return those rows, otherwise return everything
        if rows.is_some() {
            df.head(rows, call.head)?
        } else {
            df.head(Some(df.height()), call.head)?
        }
    };

    let value = Value::list(values, call.head);

    Ok(PipelineData::Value(value, None))
}

fn expression_command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: Value,
) -> Result<PipelineData, ShellError> {
    let expr = NuExpression::try_from_value(plugin, &input)?;
    let value = expr.to_value(call.head)?;

    Ok(PipelineData::Value(value, None))
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
