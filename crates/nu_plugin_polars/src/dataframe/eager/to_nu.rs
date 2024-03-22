use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};

use crate::{dataframe::values::NuExpression, PolarsDataFramePlugin};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl PluginCommand for ToNu {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars into-nu")
            .usage("Converts a dataframe or an expression into into nushell value for access and exploration.")
            .named(
                "rows",
                SyntaxShape::Number,
                "number of rows to be shown",
                Some('n'),
            )
            .switch("tail", "shows tail rows", Some('t'))
            .input_output_types(vec![
                (Type::Custom("expression".into()), Type::Any),
                (Type::Custom("dataframe".into()), Type::Table(vec![])),
            ])
            //.input_output_type(Type::Any, Type::Any)
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            dataframe_command(call, value)
        } else {
            expression_command(call, value)
        }
        .map_err(|e| e.into())
    }
}

fn examples() -> Vec<PluginExample> {
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
        PluginExample {
            description: "Shows head rows from dataframe".into(),
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars into-nu".into(),
            result: Some(Value::list(vec![rec_1, rec_2], Span::test_data())),
        },
        PluginExample {
            description: "Shows tail rows from dataframe".into(),
            example: "[[a b]; [1 2] [5 6] [3 4]] | polars into-df | polars into-nu --tail --rows 1"
                .into(),
            result: Some(Value::list(vec![rec_3], Span::test_data())),
        },
        PluginExample {
            description: "Convert a col expression into a nushell value".into(),
            example: "polars col a | polars into-nu".into(),
            result: Some(Value::test_record(record! {
                "expr" =>  Value::test_string("column"),
                "value" => Value::test_string("a"),
            })),
        },
    ]
}

fn dataframe_command(call: &EvaluatedCall, input: Value) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.get_flag("rows")?;
    let tail: bool = call.has_flag("tail")?;

    let df = NuDataFrame::try_from_value(input)?;

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

fn expression_command(call: &EvaluatedCall, input: Value) -> Result<PipelineData, ShellError> {
    let expr = NuExpression::try_from_value(input)?;
    let value = expr.to_value(call.head)?;

    Ok(PipelineData::Value(value, None))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::expressions::ExprCol;
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples_dataframe_input() {
//         test_dataframe(vec![Box::new(ToNu {})])
//     }
//
//     #[test]
//     fn test_examples_expression_input() {
//         test_dataframe(vec![Box::new(ToNu {}), Box::new(ExprCol {})])
//     }
// }
