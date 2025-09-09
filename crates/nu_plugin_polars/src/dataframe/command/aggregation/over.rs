use crate::{
    PolarsPlugin,
    dataframe::values::{NuDataFrame, NuExpression},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::df;

#[derive(Clone)]
pub struct Over;

impl PluginCommand for Over {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars over"
    }

    fn description(&self) -> &str {
        "Compute expressions over a window group defined by partition expressions."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "partition by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the partition window",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute expression over an aggregation window",
                example: r#"[[a b]; [x 2] [x 4] [y 6] [y 4]]
        | polars into-lazy
        | polars select a (polars col b | polars cumulative sum | polars over a | polars as cum_b)
        | polars collect"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => &["x", "x", "y", "y"],
                            "cum_b" => &[2, 6, 6, 10]
                        )
                        .expect("should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Compute expression over an aggregation window where partitions are defined by expressions",
                example: r#"[[a b]; [x 2] [X 4] [Y 6] [y 4]]
        | polars into-lazy
        | polars select a (polars col b | polars cumulative sum | polars over (polars col a | polars lowercase) | polars as cum_b)
        | polars collect"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => &["x", "X", "Y", "y"],
                            "cum_b" => &[2, 6, 6, 10]
                        )
                        .expect("should not fail"),
                    )
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
        let vals: Vec<Value> = call.rest(0)?;
        let expr_value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, expr_value)?;

        let input_value = input.into_value(call.head)?;

        match PolarsPluginObject::try_from_value(plugin, &input_value)? {
            PolarsPluginObject::NuExpression(expr) => {
                let expr: NuExpression = expr
                    .into_polars()
                    .over_with_options(Some(expressions), None, Default::default())
                    .map_err(|e| ShellError::GenericError {
                        error: format!("Error applying over expression: {e}"),
                        msg: "".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    })?
                    .into();
                expr.to_pipeline_data(plugin, engine, call.head)
            }
            _ => Err(cant_convert_err(
                &input_value,
                &[PolarsPluginType::NuExpression],
            )),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Over)
    }
}
