use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{DataType, lit};

#[derive(Clone)]
pub struct ExprIsIn;

impl PluginCommand for ExprIsIn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-in"
    }

    fn description(&self) -> &str {
        "Creates an is-in expression or checks to see if the elements are contained in the right series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("list", SyntaxShape::Any, "List to check if values are in")
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Creates a is-in expression",
            example: r#"let df = ([[a b]; [one 1] [two 2] [three 3]] | polars into-df);
            $df | polars with-column (polars col a | polars is-in [one two] | polars as a_in)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![
                                Value::test_string("one"),
                                Value::test_string("two"),
                                Value::test_string("three"),
                            ],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        ),
                        Column::new(
                            "a_in".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                            ],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["check", "contained", "is-contain", "match"]
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
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                ],
            )),
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
    let list: Vec<Value> = call.req(0)?;

    let values = NuDataFrame::try_from_columns(vec![Column::new("list".to_string(), list)], None)?;
    let list = values.as_series(call.head)?;

    if matches!(list.dtype(), DataType::Object(..)) {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "Cannot use a mixed list as argument".into(),
            span: call.head,
        });
    }

    // todo - at some point we should probably make this consistent with python api
    let expr: NuExpression = expr.into_polars().is_in(lit(list).implode(), true).into();
    expr.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprIsIn)
    }
}
