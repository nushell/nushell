mod arrow;
mod avro;
mod csv;
mod ndjson;
mod parquet;

use std::path::PathBuf;

use crate::{
    values::{cant_convert_err, PolarsFileType, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};

use nu_path::expand_path_with;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type,
};
use polars::error::PolarsError;

#[derive(Clone)]
pub struct SaveDF;

impl PluginCommand for SaveDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars save"
    }

    fn description(&self) -> &str {
        "Saves a dataframe to disk. For lazy dataframes a sink operation will be used if the file type supports it (parquet, ipc/arrow, csv, and ndjson)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxShape::Filepath, "Path to write to.")
            .named(
                "type",
                SyntaxShape::String,
                "File type: csv, json, parquet, arrow/ipc. If omitted, derive from file extension",
                Some('t'),
            )
            .named(
                "avro-compression",
                SyntaxShape::String,
                "Compression for avro supports deflate or snappy",
                None,
            )
            .named(
                "csv-delimiter",
                SyntaxShape::String,
                "file delimiter character",
                None,
            )
            .switch(
                "csv-no-header",
                "Indicates to exclude a header row for CSV files.",
                None,
            )
            .input_output_type(Type::Any, Type::String)
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description:
                    "Performs a streaming collect and save the output to the specified file",
                example: "[[a b];[1 2] [3 4]] | polars into-lazy | polars save test.parquet",
                result: None,
            },
            Example {
                description: "Saves dataframe to parquet file",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.parquet",
                result: None,
            },
            Example {
                description: "Saves dataframe to arrow file",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.arrow",
                result: None,
            },
            Example {
                description: "Saves dataframe to NDJSON file",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.ndjson",
                result: None,
            },
            Example {
                description: "Saves dataframe to avro file",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.avro",
                result: None,
            },
            Example {
                description: "Saves dataframe to CSV file",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr save test.csv",
                result: None,
            },
            Example {
                description: "Saves dataframe to CSV file using other delimiter",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr save test.csv --delimiter '|'",
                result: None,
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
        let value = input.into_value(call.head)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            po @ PolarsPluginObject::NuDataFrame(_) | po @ PolarsPluginObject::NuLazyFrame(_) => {
                command(plugin, engine, call, po)
            }
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
        .map_err(LabeledError::from)
    }
}

fn command(
    _plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    polars_object: PolarsPluginObject,
) -> Result<PipelineData, ShellError> {
    let spanned_file: Spanned<PathBuf> = call.req(0)?;
    let file_path = expand_path_with(&spanned_file.item, engine.get_current_dir()?, true);
    let file_span = spanned_file.span;
    let type_option: Option<(String, Span)> = call
        .get_flag("type")?
        .map(|t: Spanned<String>| (t.item, t.span))
        .or_else(|| {
            file_path
                .extension()
                .map(|e| (e.to_string_lossy().into_owned(), spanned_file.span))
        });

    match type_option {
        Some((ext, blamed)) => match PolarsFileType::from(ext.as_str()) {
            PolarsFileType::Parquet => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    parquet::command_lazy(call, lazy, &file_path, file_span)
                }
                PolarsPluginObject::NuDataFrame(ref df) => {
                    parquet::command_eager(df, &file_path, file_span)
                }
                _ => Err(unknown_file_save_error(file_span)),
            },
            PolarsFileType::Arrow => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    arrow::command_lazy(call, lazy, &file_path, file_span)
                }
                PolarsPluginObject::NuDataFrame(ref df) => {
                    arrow::command_eager(df, &file_path, file_span)
                }
                _ => Err(unknown_file_save_error(file_span)),
            },
            PolarsFileType::NdJson => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    ndjson::command_lazy(call, lazy, &file_path, file_span)
                }
                PolarsPluginObject::NuDataFrame(ref df) => {
                    ndjson::command_eager(df, &file_path, file_span)
                }
                _ => Err(unknown_file_save_error(file_span)),
            },
            PolarsFileType::Avro => match polars_object {
                PolarsPluginObject::NuLazyFrame(lazy) => {
                    let df = lazy.collect(call.head)?;
                    avro::command_eager(call, &df, &file_path, file_span)
                }
                PolarsPluginObject::NuDataFrame(ref df) => {
                    avro::command_eager(call, df, &file_path, file_span)
                }
                _ => Err(unknown_file_save_error(file_span)),
            },
            PolarsFileType::Csv => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    csv::command_lazy(call, lazy, &file_path, file_span)
                }
                PolarsPluginObject::NuDataFrame(ref df) => {
                    csv::command_eager(call, df, &file_path, file_span)
                }
                _ => Err(unknown_file_save_error(file_span)),
            },
            _ => Err(PolarsFileType::build_unsupported_error(
                &ext,
                &[
                    PolarsFileType::Parquet,
                    PolarsFileType::Csv,
                    PolarsFileType::Arrow,
                    PolarsFileType::NdJson,
                    PolarsFileType::Avro,
                ],
                blamed,
            )),
        },
        None => Err(ShellError::FileNotFoundCustom {
            msg: "File without extension".into(),
            span: spanned_file.span,
        }),
    }?;

    Ok(PipelineData::empty())
}

pub(crate) fn polars_file_save_error(e: PolarsError, span: Span) -> ShellError {
    ShellError::GenericError {
        error: format!("Error saving file: {e}"),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}

pub fn unknown_file_save_error(span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Could not save file for unknown reason".into(),
        msg: "".into(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}

#[cfg(test)]
pub(crate) mod test {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{Span, Value};
    use uuid::Uuid;

    use crate::PolarsPlugin;

    fn test_save(cmd: &'static str, extension: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        tmp_file.push(format!("{}.{}", Uuid::new_v4(), extension));
        let tmp_file_str = tmp_file.to_str().expect("should be able to get file path");

        let cmd = format!("{cmd} {tmp_file_str}");
        let mut plugin_test = PluginTest::new("polars", PolarsPlugin::default().into())?;
        plugin_test.engine_state_mut().add_env_var(
            "PWD".to_string(),
            Value::string(
                tmp_dir
                    .path()
                    .to_str()
                    .expect("should be able to get path")
                    .to_owned(),
                Span::test_data(),
            ),
        );
        let _pipeline_data = plugin_test.eval(&cmd)?;

        assert!(tmp_file.exists());

        Ok(())
    }

    pub fn test_lazy_save(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
        test_save(
            "[[a b]; [1 2] [3 4]] | polars into-lazy | polars save",
            extension,
        )
    }

    pub fn test_eager_save(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
        test_save(
            "[[a b]; [1 2] [3 4]] | polars into-df | polars save",
            extension,
        )
    }
}
