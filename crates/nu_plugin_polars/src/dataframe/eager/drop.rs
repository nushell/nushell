use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};

use crate::PolarsDataFramePlugin;

use super::super::values::utils::convert_columns;
use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropDF;

impl PluginCommand for DropDF {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars drop")
            .usage("Creates a new dataframe by dropping the selected columns.")
            .rest("rest", SyntaxShape::Any, "column names to be dropped")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(vec![PluginExample {
                description: "drop column a".into(),
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars drop a".into(),
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            }])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let columns: Vec<Value> = call.rest(0)?;
    let (col_string, col_span) = convert_columns(columns, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let new_df = col_string
        .first()
        .ok_or_else(|| ShellError::GenericError {
            error: "Empty names list".into(),
            msg: "No column names were found".into(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })
        .and_then(|col| {
            df.as_ref()
                .drop(&col.item)
                .map_err(|e| ShellError::GenericError {
                    error: "Error dropping column".into(),
                    msg: e.to_string(),
                    span: Some(col.span),
                    help: None,
                    inner: vec![],
                })
        })?;

    // If there are more columns in the drop selection list, these
    // are added from the resulting dataframe
    let polars_df = col_string.iter().skip(1).try_fold(new_df, |new_df, col| {
        new_df
            .drop(&col.item)
            .map_err(|e| ShellError::GenericError {
                error: "Error dropping column".into(),
                msg: e.to_string(),
                span: Some(col.span),
                help: None,
                inner: vec![],
            })
    })?;

    let final_df = NuDataFrame::new(df.from_lazy, polars_df);

    Ok(PipelineData::Value(
        final_df.insert_cache(engine)?.into_value(call.head),
        None,
    ))
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(DropDF {})])
//     }
// }
