use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame, NuExpression},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};
use polars::df;

#[derive(Clone)]
pub struct StrSplit;

impl PluginCommand for StrSplit {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars str-split"
    }

    fn description(&self) -> &str {
        "Split the string by a substring. The resulting dtype is list<str>."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("expr", SyntaxShape::Any, "Separator expression")
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split the string by comma, then create a new row for each string",
            example: r#"[[a]; ["one,two,three"]] | polars into-df 
                | polars select (polars col a | polars str-split "," | polars explode) 
                | polars collect"#,
            result: Some(
                NuDataFrame::from(
                    df!(
                    "a" => ["one", "two", "three"]
                    )
                    .expect("Should be able to create a dataframe"),
                )
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let separator = call.req::<Spanned<Value>>(0).and_then(|sep| {
            let sep_expr = NuExpression::try_from_value(plugin, &sep.item)?;
            Ok(Spanned {
                item: sep_expr,
                span: sep.span,
            })
        })?;

        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;
        let res: NuExpression = expr
            .into_polars()
            .str()
            .split(separator.item.into_polars())
            .into();
        res.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use nu_protocol::ShellError;

    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&StrSplit)
    }
}
