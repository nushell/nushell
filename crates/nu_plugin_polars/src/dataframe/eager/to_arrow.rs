use std::{fs::File, path::PathBuf};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{
    Category, PipelineData, PluginExample, PluginSignature, ShellError, Spanned, SyntaxShape, Type,
    Value,
};
use polars::prelude::{IpcWriter, SerWriter};

use crate::PolarsDataFramePlugin;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToArrow;

impl PluginCommand for ToArrow {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars to-arrow")
            .usage("Saves dataframe to arrow file.")
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(vec![PluginExample {
                description: "Saves dataframe to arrow file".into(),
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars to-arrow test.arrow"
                    .into(),
                result: None,
            }])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(call, input).map_err(|e| e.into())
    }
}

fn command(call: &EvaluatedCall, input: PipelineData) -> Result<PipelineData, ShellError> {
    let file_name: Spanned<PathBuf> = call.req(0)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let mut file = File::create(&file_name.item).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_name.span),
        help: None,
        inner: vec![],
    })?;

    IpcWriter::new(&mut file)
        .finish(&mut df.to_polars())
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
