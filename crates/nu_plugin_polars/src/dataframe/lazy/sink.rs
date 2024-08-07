use std::path::PathBuf;

use crate::{
    values::{cant_convert_err, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};

use super::super::values::NuLazyFrame;

use nu_path::expand_path_with;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use polars::error::PolarsError;
use polars_io::{
    csv::write::CsvWriterOptions, ipc::IpcWriterOptions, json::JsonWriterOptions,
    parquet::write::ParquetWriteOptions,
};

#[derive(Clone)]
pub struct Sink;

impl PluginCommand for Sink {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars sink"
    }

    fn usage(&self) -> &str {
        "Streams a collect result to a file. This is useful if the result is too large for memory. Supports parquet, ipc/arrow, csv, and json formats."
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
            .input_output_type(Type::Any, Type::String)
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Collect and save the output to the specified file",
            example: "[[a b];[1 2] [3 4]] | polars into-lazy | polars sink /tmp/foo.parquet",
            result: None,
        }]
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
            PolarsPluginObject::NuDataFrame(df) => command(plugin, engine, call, df.lazy()),
            PolarsPluginObject::NuLazyFrame(lazy) => command(plugin, engine, call, lazy),
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
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let spanned_file: Spanned<PathBuf> = call.req(0)?;
    let file_path = expand_path_with(&spanned_file.item, engine.get_current_dir()?, true);
    let file_span = spanned_file.span;
    let type_option: Option<Spanned<String>> = call.get_flag("type")?;
    let type_id = match &type_option {
        Some(ref t) => Some((t.item.to_owned(), "Invalid type", t.span)),
        None => file_path.extension().map(|e| {
            (
                e.to_string_lossy().into_owned(),
                "Invalid extension",
                spanned_file.span,
            )
        }),
    };

    let polars_df = lazy.to_polars();

    match type_id {
        Some((e, msg, blamed)) => match e.as_str() {
            "parquet" | "parq" => polars_df
                .sink_parquet(&file_path, ParquetWriteOptions::default())
                .map_err(|e| file_save_error(e, file_span))?,
            "csv" => polars_df
                .sink_csv(&file_path, CsvWriterOptions::default())
                .map_err(|e| file_save_error(e, file_span))?,
            "ipc" | "arrow" => polars_df
                .sink_ipc(&file_path, IpcWriterOptions::default())
                .map_err(|e| file_save_error(e, file_span))?,
            "json" | "jsonl" | "ndjson" => polars_df
                .sink_json(&file_path, JsonWriterOptions::default())
                .map_err(|e| file_save_error(e, file_span))?,
            _ => Err(ShellError::FileNotFoundCustom {
                msg: format!("{msg}. Supported values: csv, tsv, parquet, ipc, arrow, json, jsonl"),
                span: blamed,
            })?,
        },
        None => Err(ShellError::FileNotFoundCustom {
            msg: "File without extension".into(),
            span: spanned_file.span,
        })?,
    };
    let file_value = Value::string(format!("saved {:?}", &file_path), file_span);

    Ok(PipelineData::Value(
        Value::list(vec![file_value], call.head),
        None,
    ))
}

fn file_save_error(e: PolarsError, span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Error saving file".into(),
        msg: e.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}

#[cfg(test)]
pub mod test {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{Span, Value};
    use uuid::Uuid;

    use crate::PolarsPlugin;

    pub fn test_sink(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let mut tmp_file = tmp_dir.path().to_owned();
        tmp_file.push(format!("{}.{}", Uuid::new_v4(), extension));
        let tmp_file_str = tmp_file.to_str().expect("should be able to get file path");

        let cmd = format!(
            "[[a b]; [1 2] [3 4]] | polars into-lazy | polars sink {}",
            tmp_file_str
        );
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
        let pipeline_data = plugin_test.eval(&cmd)?;

        assert!(tmp_file.exists());

        let value = pipeline_data.into_value(Span::test_data())?;
        let list = value.as_list()?;
        assert_eq!(list.len(), 1);
        let msg = list.first().expect("should have a value").as_str()?;
        assert!(msg.contains("saved"));

        Ok(())
    }

    #[test]
    pub fn test_to_parquet() -> Result<(), Box<dyn std::error::Error>> {
        test_sink("parquet")
    }

    #[test]
    pub fn test_to_ipc() -> Result<(), Box<dyn std::error::Error>> {
        test_sink("ipc")
    }

    #[test]
    pub fn test_to_csv() -> Result<(), Box<dyn std::error::Error>> {
        test_sink("csv")
    }

    #[test]
    pub fn test_to_json() -> Result<(), Box<dyn std::error::Error>> {
        test_sink("ndjson")
    }
}
