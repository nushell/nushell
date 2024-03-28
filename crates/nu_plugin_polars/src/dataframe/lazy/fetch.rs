use crate::dataframe::values::{Column, NuDataFrame};
use crate::values::{CustomValueSupport, NuLazyFrame};
use crate::{values::PolarsPluginObject, Cacheable, PolarsPlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LazyFetch;

impl PluginCommand for LazyFetch {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars fetch"
    }

    fn usage(&self) -> &str {
        "Collects the lazyframe to the selected rows."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "rows",
                SyntaxShape::Int,
                "number of rows to be fetched from lazyframe",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Fetch a rows from the dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars fetch 2",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(6), Value::test_int(4)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(2)],
                        ),
                    ],
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
        let rows: i64 = call.req(0)?;

        let lazy: NuLazyFrame =
            match PolarsPluginObject::try_from_pipeline(plugin, input, call.head)? {
                PolarsPluginObject::NuDataFrame(df) => Ok::<NuLazyFrame, LabeledError>(df.lazy()),
                PolarsPluginObject::NuLazyFrame(lazy) => Ok(lazy),
                _ => return Err(LabeledError::new("A Dataframe or LazyFrame is required")),
            }?;

        let eager: NuDataFrame = lazy
            .to_polars()
            .fetch(rows as usize)
            .map_err(|e| ShellError::GenericError {
                error: "Error fetching rows".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        Ok(PipelineData::Value(
            eager.cache(plugin, engine)?.into_value(call.head),
            None,
        ))
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(LazyFetch {})])
//     }
// }
