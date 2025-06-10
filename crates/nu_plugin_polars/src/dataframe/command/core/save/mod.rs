mod arrow;
mod avro;
mod csv;
mod ndjson;
mod parquet;

use std::{path::PathBuf, sync::Arc};

use crate::{
    PolarsPlugin,
    command::core::resource::Resource,
    values::{PolarsFileType, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use log::debug;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, DataSource, Example, LabeledError, PipelineData, PipelineMetadata, ShellError,
    Signature, Span, Spanned, SyntaxShape, Type,
    shell_error::{self, io::IoError},
};
use polars::{error::PolarsError, prelude::SinkTarget};

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
            .required("path", SyntaxShape::String, "Path or cloud url to write to")
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
                description: "Performs a streaming collect and save the output to the specified file",
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
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.csv",
                result: None,
            },
            Example {
                description: "Saves dataframe to CSV file using other delimiter",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars save test.csv --csv-delimiter '|'",
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
        let spanned_file: Spanned<String> = call.req(0)?;
        debug!("file: {}", spanned_file.item);

        let metadata = input.metadata();
        let value = input.into_value(call.head)?;

        check_writing_into_source_file(
            metadata.as_ref(),
            &spanned_file.as_ref().map(PathBuf::from),
        )?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            po @ PolarsPluginObject::NuDataFrame(_) | po @ PolarsPluginObject::NuLazyFrame(_) => {
                command(plugin, engine, call, po, spanned_file)
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
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    polars_object: PolarsPluginObject,
    spanned_file: Spanned<String>,
) -> Result<PipelineData, ShellError> {
    let resource = Resource::new(plugin, engine, &spanned_file)?;
    let type_option: Option<(String, Span)> = call
        .get_flag("type")?
        .map(|t: Spanned<String>| (t.item, t.span))
        .or_else(|| resource.extension.clone().map(|e| (e, resource.span)));
    debug!("resource: {resource:?}");

    match type_option {
        Some((ext, blamed)) => match PolarsFileType::from(ext.as_str()) {
            PolarsFileType::Parquet => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    parquet::command_lazy(call, lazy, resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) if resource.cloud_options.is_some() => {
                    parquet::command_lazy(call, &df.lazy(), resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) => parquet::command_eager(df, resource),
                _ => Err(unknown_file_save_error(resource.span)),
            },
            PolarsFileType::Arrow => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    arrow::command_lazy(call, lazy, resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) if resource.cloud_options.is_some() => {
                    arrow::command_lazy(call, &df.lazy(), resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) => arrow::command_eager(df, resource),
                _ => Err(unknown_file_save_error(resource.span)),
            },
            PolarsFileType::NdJson => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    ndjson::command_lazy(call, lazy, resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) if resource.cloud_options.is_some() => {
                    ndjson::command_lazy(call, &df.lazy(), resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) => ndjson::command_eager(df, resource),
                _ => Err(unknown_file_save_error(resource.span)),
            },
            PolarsFileType::Avro => match polars_object {
                _ if resource.cloud_options.is_some() => Err(ShellError::GenericError {
                    error: "Cloud URLS are not supported with Avro".into(),
                    msg: "".into(),
                    span: call.get_flag_span("eager"),
                    help: Some("Remove flag".into()),
                    inner: vec![],
                }),
                PolarsPluginObject::NuLazyFrame(lazy) => {
                    let df = lazy.collect(call.head)?;
                    avro::command_eager(call, &df, resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) => avro::command_eager(call, df, resource),
                _ => Err(unknown_file_save_error(resource.span)),
            },
            PolarsFileType::Csv => match polars_object {
                PolarsPluginObject::NuLazyFrame(ref lazy) => {
                    csv::command_lazy(call, lazy, resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) if resource.cloud_options.is_some() => {
                    csv::command_lazy(call, &df.lazy(), resource)
                }
                PolarsPluginObject::NuDataFrame(ref df) => csv::command_eager(call, df, resource),
                _ => Err(unknown_file_save_error(resource.span)),
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
        None => Err(ShellError::Io(IoError::new_with_additional_context(
            shell_error::io::ErrorKind::FileNotFound,
            resource.span,
            Some(PathBuf::from(resource.path)),
            "File without extension",
        ))),
    }?;

    Ok(PipelineData::empty())
}

fn check_writing_into_source_file(
    metadata: Option<&PipelineMetadata>,
    dest: &Spanned<PathBuf>,
) -> Result<(), ShellError> {
    let Some(DataSource::FilePath(source)) = metadata.map(|meta| &meta.data_source) else {
        return Ok(());
    };

    if &dest.item == source {
        return Err(write_into_source_error(dest.span));
    }

    Ok(())
}

fn write_into_source_error(span: Span) -> ShellError {
    polars_file_save_error(
        PolarsError::InvalidOperation("attempted to save into source".into()),
        span,
    )
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

pub(crate) fn sink_target_from_string(path: String) -> SinkTarget {
    let path = PathBuf::from(path);
    let target = SinkTarget::Path(Arc::new(path));
    debug!("Sink target: {target:?}");
    target
}

#[cfg(test)]
pub(crate) mod test {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{Span, Value};
    use tempfile::TempDir;
    use uuid::Uuid;

    use crate::PolarsPlugin;

    fn tmp_dir_sandbox() -> Result<(TempDir, PluginTest), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let mut plugin_test = PluginTest::new("polars", PolarsPlugin::new()?.into())?;
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

        Ok((tmp_dir, plugin_test))
    }

    fn test_save(cmd: &'static str, extension: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (tmp_dir, mut plugin_test) = tmp_dir_sandbox()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        tmp_file.push(format!("{}.{}", Uuid::new_v4(), extension));
        let tmp_file_str = tmp_file.to_str().expect("should be able to get file path");

        let cmd = format!("{cmd} {tmp_file_str}");
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

    #[test]
    fn test_write_to_source_guard() -> Result<(), Box<dyn std::error::Error>> {
        let (tmp_dir, mut plugin_test) = tmp_dir_sandbox()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        dbg!(&tmp_dir);
        tmp_file.push(format!("{}.{}", Uuid::new_v4(), "parquet"));
        let tmp_file_str = tmp_file.to_str().expect("Should be able to get file path");

        let _setup = plugin_test.eval(&format!(
            "[1 2 3] | polars into-df | polars save {tmp_file_str}",
        ))?;

        let output = plugin_test.eval(&format!(
            "polars open {tmp_file_str} | polars save {tmp_file_str}"
        ));

        assert!(output.is_err_and(|e| {
            e.to_string()
                .contains("Error saving file: attempted to save into source")
        }));

        Ok(())
    }
}
