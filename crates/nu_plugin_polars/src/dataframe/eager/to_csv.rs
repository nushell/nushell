use std::{fs::File, path::PathBuf};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Spanned,
    SyntaxShape, Type, Value,
};
use polars::prelude::{CsvWriter, SerWriter};

use crate::{CustomValueSupport, PolarsPlugin};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToCSV;

impl PluginCommand for ToCSV {
    type Plugin = PolarsPlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars to-csv")
            .usage("Saves dataframe to CSV file.")
            .required("file", SyntaxShape::Filepath, "file path to save dataframe")
            .named(
                "delimiter",
                SyntaxShape::String,
                "file delimiter character",
                Some('d'),
            )
            .switch("no-header", "Indicates if file doesn't have header", None)
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(vec![
                PluginExample {
                    description: "Saves dataframe to CSV file".into(),
                    example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv".into(),
                    result: None,
                },
                PluginExample {
                    description: "Saves dataframe to CSV file using other delimiter".into(),
                    example:
                        "[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv --delimiter '|'"
                            .into(),
                    result: None,
                },
            ])
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, call, input).map_err(|e| e.into())
    }
}

fn command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let file_name: Spanned<PathBuf> = call.req(0)?;
    let delimiter: Option<Spanned<String>> = call.get_flag("delimiter")?;
    let no_header: bool = call.has_flag("no-header")?;

    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let mut file = File::create(&file_name.item).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_name.span),
        help: None,
        inner: vec![],
    })?;

    let writer = CsvWriter::new(&mut file);

    let writer = if no_header {
        writer.include_header(false)
    } else {
        writer.include_header(true)
    };

    let mut writer = match delimiter {
        None => writer,
        Some(d) => {
            if d.item.len() != 1 {
                return Err(ShellError::GenericError {
                    error: "Incorrect delimiter".into(),
                    msg: "Delimiter has to be one char".into(),
                    span: Some(d.span),
                    help: None,
                    inner: vec![],
                });
            } else {
                let delimiter = match d.item.chars().next() {
                    Some(d) => d as u8,
                    None => unreachable!(),
                };

                writer.with_separator(delimiter)
            }
        }
    };

    writer
        .finish(&mut df.to_polars())
        .map_err(|e| ShellError::GenericError {
            error: "Error writing to file".into(),
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
