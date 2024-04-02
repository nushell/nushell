use crate::{
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{is_in, IntoSeries};

#[derive(Clone)]
pub struct IsIn;

impl PluginCommand for IsIn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars series-is-in"
    }

    fn usage(&self) -> &str {
        "Checks if elements from a series are contained in right series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "right series")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Checks if elements from a series are contained in right series",
            example: r#"let other = ([1 3 6] | polars into-df);
    [5 6 6 6 8 8 8] | polars into-df | polars is-in $other"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "is_in".to_string(),
                        vec![
                            Value::test_bool(false),
                            Value::test_bool(true),
                            Value::test_bool(true),
                            Value::test_bool(true),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                        ],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df =
        NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?.as_series(call.head)?;

    let other_value: Value = call.req(0)?;
    let other_span = other_value.span();
    let other_df = NuDataFrame::try_from_value(plugin, &other_value)?;
    let other = other_df.as_series(other_span)?;

    let mut res = is_in(&df, &other)
        .map_err(|e| ShellError::GenericError {
            error: "Error finding in other".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    res.rename("is_in");

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    to_pipeline_data(plugin, engine, call.head, df)
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(IsIn {})])
//     }
// }
