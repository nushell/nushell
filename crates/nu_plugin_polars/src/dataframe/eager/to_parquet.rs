use std::{fs::File, path::PathBuf};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value,
};
use polars::prelude::ParquetWriter;

use crate::PolarsPlugin;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToParquet;

impl PluginCommand for ToParquet {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars to-parquet"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to parquet file."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to parquet file",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars to-parquet test.parquet",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let file_name: Spanned<PathBuf> = call.req(0)?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let file = File::create(&file_name.item).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_name.span),
        help: None,
        inner: vec![],
    })?;
    let mut polars_df = df.to_polars();
    ParquetWriter::new(file)
        .finish(&mut polars_df)
        .map_err(|e| ShellError::GenericError {
            error: "Error saving file".into(),
            msg: e.to_string(),
            span: Some(file_name.span),
            help: None,
            inner: vec![],
        })?;

    let file_value = Value::string(format!("saved {:?}", &file_name.item), file_name.span);

    Ok(PipelineData::Value(
        Value::list(vec![file_value], call.head),
        None,
    ))
}
