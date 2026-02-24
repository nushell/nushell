use nu_plugin::PluginCommand;
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape,
};
use polars::df;

use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType,
        cant_convert_err,
    },
};

pub struct Entropy;

impl PluginCommand for Entropy {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars entropy"
    }

    fn description(&self) -> &str {
        "Compute the entropy as `-sum(pk * log(pk))` where `pk` are discrete probabilities."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "base",
                SyntaxShape::Float,
                "Given base, defaults to e.",
                None,
            )
            .named(
                "normalize",
                SyntaxShape::Boolean,
                "Normalize pk if it doesn’t sum to 1. Default to true.",
                None,
            )
            .input_output_types(vec![(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the entropy of a column expression",
                example: "[[a]; [1] [2] [3]] | polars into-df | polars select (polars col a | polars entropy --base 2) | polars collect",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "a" => [1.4591479170272448f64]
                        )
                        .expect("should be able to create a dataframe"),
                    )
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Compute the entropy of a column expression without normalization",
                example: "[[a]; [1] [2] [3]] | polars into-df | polars select (polars col a | polars entropy --base 2 --normalize false) | polars collect",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "a" => [-6.754887502163469f64]
                        )
                        .expect("should be able to create a dataframe"),
                    )
                    .into_value(Span::unknown()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
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
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let base = call.get_flag::<f64>("base")?.unwrap_or(std::f64::consts::E);
    let normalize = call.get_flag::<bool>("normalize")?.unwrap_or(true);

    let expr: NuExpression = expr.into_polars().entropy(base, normalize).into();

    expr.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Entropy)
    }
}
