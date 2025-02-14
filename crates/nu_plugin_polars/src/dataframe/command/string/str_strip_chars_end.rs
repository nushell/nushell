use crate::{
    values::{
        cant_convert_err, CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType,
    },
    PolarsPlugin,
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct StrStripCharsEnd;

impl PluginCommand for StrStripCharsEnd {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-strip-chars-end"
    }

    fn description(&self) -> &str {
        "Strips specified characters from the end of strings in a column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("pattern", SyntaxShape::String, "Characters to strip from the end")
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                )
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Strip characters from end of strings in a column",
                example: r#"[[text]; ["hello!!!"] ["world!!!"] ["test!!!"]] | polars into-df | polars select (polars col text | polars str-strip-chars-end "!") | polars collect"#,
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let pattern: String = call.req(0)?;

    let pattern_expr = polars::prelude::lit(pattern);
    
    let res: NuExpression = expr
        .into_polars()
        .str()
        .strip_chars_end(pattern_expr)
        .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&StrStripCharsEnd)
    }
}
