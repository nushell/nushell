use super::super::values::{Column, NuDataFrame};
use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::CustomValueSupport,
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct WithColumn;

impl PluginCommand for WithColumn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars with-column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "series or expressions",
                SyntaxShape::Any,
                "series to be added or expressions used to define the new columns",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example: r#"[[a b]; [1 2] [3 4]]
    | polars into-df
    | polars with-column [
        ((polars col a) * 2 | polars as "c")
        ((polars col a) * 3 | polars as "d")
      ]
    | polars collect"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                        Column::new(
                            "c".to_string(),
                            vec![Value::test_int(2), Value::test_int(6)],
                        ),
                        Column::new(
                            "d".to_string(),
                            vec![Value::test_int(3), Value::test_int(9)],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
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
        let value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)?;
        command_lazy(plugin, engine, call, lazy).map_err(LabeledError::from)
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let vals: Vec<Value> = call.rest(0)?;
    let value = Value::list(vals, call.head);
    let expressions = NuExpression::extract_exprs(plugin, value)?;
    let lazy: NuLazyFrame = lazy.to_polars().with_columns(&expressions).into();
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&WithColumn)
    }
}
