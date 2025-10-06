use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct StrStripChars;

impl PluginCommand for StrStripChars {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-strip-chars"
    }

    fn description(&self) -> &str {
        "Strips specified characters from strings in a column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "pattern",
                SyntaxShape::Any,
                "Characters to strip as either a string or polars expression",
            )
            .switch("start", "Strip from start of strings only", Some('s'))
            .switch("end", "Strip from end of strings only", Some('e'))
            .input_output_types(vec![(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Strip characters from both ends of strings in a column",
                example: r#"[[text]; ["!!!hello!!!"] ["!!!world!!!"] ["!!!test!!!"]] | polars into-df | polars select (polars col text | polars str-strip-chars "!") | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "text".to_string(),
                            vec![
                                Value::test_string("hello"),
                                Value::test_string("world"),
                                Value::test_string("test"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Strip characters from both ends of strings in a column using an expression",
                example: r#"[[text]; ["!!!hello!!!"] ["!!!world!!!"] ["!!!test!!!"]] | polars into-df | polars select (polars col text | polars str-strip-chars (polars lit "!")) | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "text".to_string(),
                            vec![
                                Value::test_string("hello"),
                                Value::test_string("world"),
                                Value::test_string("test"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Strip characters from end of strings in a column",
                example: r#"[[text]; ["hello!!!"] ["world!!!"] ["test!!!"]] | polars into-df | polars select (polars col text | polars str-strip-chars --end "!") | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "text".to_string(),
                            vec![
                                Value::test_string("hello"),
                                Value::test_string("world"),
                                Value::test_string("test"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Strip characters from start of strings in a column",
                example: r#"[[text]; ["!!!hello"] ["!!!world"] ["!!!test"]] | polars into-df | polars select (polars col text | polars str-strip-chars --start "!") | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "text".to_string(),
                            vec![
                                Value::test_string("hello"),
                                Value::test_string("world"),
                                Value::test_string("test"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
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
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let pattern_expr: Expr = call
        .req::<Value>(0)
        .and_then(|ref v| NuExpression::try_from_value(plugin, v))
        .or(call
            .req::<String>(0)
            .map(polars::prelude::lit)
            .map(NuExpression::from))?
        .into_polars();
    let strip_start = call.has_flag("start")?;
    let strip_end = call.has_flag("end")?;

    let res: NuExpression = if strip_start {
        // Use strip_chars_start when --start flag is provided
        expr.into_polars()
            .str()
            .strip_chars_start(pattern_expr)
            .into()
    } else if strip_end {
        // Use strip_chars_end when --end flag is provided
        expr.into_polars()
            .str()
            .strip_chars_end(pattern_expr)
            .into()
    } else {
        // Use strip_chars when no flags are provided (both ends)
        expr.into_polars().str().strip_chars(pattern_expr).into()
    };

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&StrStripChars)
    }
}
