use crate::{
    dataframe::values::NuSchema,
    values::{CustomValueSupport, NuLazyFrame},
    PolarsPlugin,
};
use nu_path::expand_path_with;

use super::super::values::NuDataFrame;
use nu_plugin::PluginCommand;
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use polars::prelude::{
    CsvEncoding, CsvReader, IpcReader, JsonFormat, JsonReader, LazyCsvReader, LazyFileListReader,
    LazyFrame, ParquetReader, ScanArgsIpc, ScanArgsParquet, SerReader,
};

use polars_io::{avro::AvroReader, prelude::ParallelStrategy, HiveOptions};

#[derive(Clone)]
pub struct OpenDataFrame;

impl PluginCommand for OpenDataFrame {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars open"
    }

    fn usage(&self) -> &str {
        "Opens CSV, JSON, JSON lines, arrow, avro, or parquet file to create dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "file",
                SyntaxShape::Filepath,
                "file path to load values from",
            )
            .switch("lazy", "creates a lazy dataframe", Some('l'))
            .named(
                "type",
                SyntaxShape::String,
                "File type: csv, tsv, json, parquet, arrow, avro. If omitted, derive from file extension",
                Some('t'),
            )
            .named(
                "delimiter",
                SyntaxShape::String,
                "file delimiter character. CSV file",
                Some('d'),
            )
            .switch(
                "no-header",
                "Indicates if file doesn't have header. CSV file",
                None,
            )
            .named(
                "infer-schema",
                SyntaxShape::Number,
                "Number of rows to infer the schema of the file. CSV file",
                None,
            )
            .named(
                "skip-rows",
                SyntaxShape::Number,
                "Number of rows to skip from file. CSV file",
                None,
            )
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Columns to be selected from csv file. CSV and Parquet file",
                None,
            )
            .named(
                "schema",
                SyntaxShape::Record(vec![]),
                r#"Polars Schema in format [{name: str}]. CSV, JSON, and JSONL files"#,
                Some('s')
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a file name and creates a dataframe",
            example: "polars open test.csv",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, LabeledError> {
        command(plugin, engine, call).map_err(|e| e.into())
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
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

    match type_id {
        Some((e, msg, blamed)) => match e.as_str() {
            "csv" | "tsv" => from_csv(plugin, engine, call, &file_path, file_span),
            "parquet" | "parq" => from_parquet(plugin, engine, call, &file_path, file_span),
            "ipc" | "arrow" => from_ipc(plugin, engine, call, &file_path, file_span),
            "json" => from_json(plugin, engine, call, &file_path, file_span),
            "jsonl" => from_jsonl(plugin, engine, call, &file_path, file_span),
            "avro" => from_avro(plugin, engine, call, &file_path, file_span),
            _ => Err(ShellError::FileNotFoundCustom {
                msg: format!(
                    "{msg}. Supported values: csv, tsv, parquet, ipc, arrow, json, jsonl, avro"
                ),
                span: blamed,
            }),
        },
        None => Err(ShellError::FileNotFoundCustom {
            msg: "File without extension".into(),
            span: spanned_file.span,
        }),
    }
    .map(|value| PipelineData::Value(value, None))
}

fn from_parquet(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    if call.has_flag("lazy")? {
        let file: String = call.req(0)?;
        let args = ScanArgsParquet {
            n_rows: None,
            cache: true,
            parallel: ParallelStrategy::Auto,
            rechunk: false,
            row_index: None,
            low_memory: false,
            cloud_options: None,
            use_statistics: false,
            hive_options: HiveOptions::default(),
        };

        let df: NuLazyFrame = LazyFrame::scan_parquet(file, args)
            .map_err(|e| ShellError::GenericError {
                error: "Parquet reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    } else {
        let columns: Option<Vec<String>> = call.get_flag("columns")?;

        let r = File::open(file_path).map_err(|e| ShellError::GenericError {
            error: "Error opening file".into(),
            msg: e.to_string(),
            span: Some(file_span),
            help: None,
            inner: vec![],
        })?;
        let reader = ParquetReader::new(r);

        let reader = match columns {
            None => reader,
            Some(columns) => reader.with_columns(Some(columns)),
        };

        let df: NuDataFrame = reader
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "Parquet reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    }
}

fn from_avro(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    let columns: Option<Vec<String>> = call.get_flag("columns")?;

    let r = File::open(file_path).map_err(|e| ShellError::GenericError {
        error: "Error opening file".into(),
        msg: e.to_string(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;
    let reader = AvroReader::new(r);

    let reader = match columns {
        None => reader,
        Some(columns) => reader.with_columns(Some(columns)),
    };

    let df: NuDataFrame = reader
        .finish()
        .map_err(|e| ShellError::GenericError {
            error: "Avro reader error".into(),
            msg: format!("{e:?}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into();

    df.cache_and_to_value(plugin, engine, call.head)
}

fn from_ipc(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    if call.has_flag("lazy")? {
        let file: String = call.req(0)?;
        let args = ScanArgsIpc {
            n_rows: None,
            cache: true,
            rechunk: false,
            row_index: None,
            memory_map: true,
            cloud_options: None,
        };

        let df: NuLazyFrame = LazyFrame::scan_ipc(file, args)
            .map_err(|e| ShellError::GenericError {
                error: "IPC reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    } else {
        let columns: Option<Vec<String>> = call.get_flag("columns")?;

        let r = File::open(file_path).map_err(|e| ShellError::GenericError {
            error: "Error opening file".into(),
            msg: e.to_string(),
            span: Some(file_span),
            help: None,
            inner: vec![],
        })?;
        let reader = IpcReader::new(r);

        let reader = match columns {
            None => reader,
            Some(columns) => reader.with_columns(Some(columns)),
        };

        let df: NuDataFrame = reader
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "IPC reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    }
}

fn from_json(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    let file = File::open(file_path).map_err(|e| ShellError::GenericError {
        error: "Error opening file".into(),
        msg: e.to_string(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;
    let maybe_schema = call
        .get_flag("schema")?
        .map(|schema| NuSchema::try_from(&schema))
        .transpose()?;

    let buf_reader = BufReader::new(file);
    let reader = JsonReader::new(buf_reader);

    let reader = match maybe_schema {
        Some(schema) => reader.with_schema(schema.into()),
        None => reader,
    };

    let df: NuDataFrame = reader
        .finish()
        .map_err(|e| ShellError::GenericError {
            error: "Json reader error".into(),
            msg: format!("{e:?}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into();

    df.cache_and_to_value(plugin, engine, call.head)
}

fn from_jsonl(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    let infer_schema: Option<usize> = call.get_flag("infer-schema")?;
    let maybe_schema = call
        .get_flag("schema")?
        .map(|schema| NuSchema::try_from(&schema))
        .transpose()?;
    let file = File::open(file_path).map_err(|e| ShellError::GenericError {
        error: "Error opening file".into(),
        msg: e.to_string(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;

    let buf_reader = BufReader::new(file);
    let reader = JsonReader::new(buf_reader)
        .with_json_format(JsonFormat::JsonLines)
        .infer_schema_len(infer_schema);

    let reader = match maybe_schema {
        Some(schema) => reader.with_schema(schema.into()),
        None => reader,
    };

    let df: NuDataFrame = reader
        .finish()
        .map_err(|e| ShellError::GenericError {
            error: "Json lines reader error".into(),
            msg: format!("{e:?}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into();

    df.cache_and_to_value(plugin, engine, call.head)
}

fn from_csv(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    let delimiter: Option<Spanned<String>> = call.get_flag("delimiter")?;
    let no_header: bool = call.has_flag("no-header")?;
    let infer_schema: Option<usize> = call.get_flag("infer-schema")?;
    let skip_rows: Option<usize> = call.get_flag("skip-rows")?;
    let columns: Option<Vec<String>> = call.get_flag("columns")?;

    let maybe_schema = call
        .get_flag("schema")?
        .map(|schema| NuSchema::try_from(&schema))
        .transpose()?;

    if call.has_flag("lazy")? {
        let csv_reader = LazyCsvReader::new(file_path);

        let csv_reader = match delimiter {
            None => csv_reader,
            Some(d) => {
                if d.item.len() != 1 {
                    return Err(ShellError::GenericError {
                        error: "Incorrect delimiter".into(),
                        msg: "Delimiter has to be one character".into(),
                        span: Some(d.span),
                        help: None,
                        inner: vec![],
                    });
                } else {
                    let delimiter = match d.item.chars().next() {
                        Some(d) => d as u8,
                        None => unreachable!(),
                    };
                    csv_reader.with_separator(delimiter)
                }
            }
        };

        let csv_reader = csv_reader.has_header(!no_header);

        let csv_reader = match maybe_schema {
            Some(schema) => csv_reader.with_schema(Some(schema.into())),
            None => csv_reader,
        };

        let csv_reader = match infer_schema {
            None => csv_reader,
            Some(r) => csv_reader.with_infer_schema_length(Some(r)),
        };

        let csv_reader = match skip_rows {
            None => csv_reader,
            Some(r) => csv_reader.with_skip_rows(r),
        };

        let df: NuLazyFrame = csv_reader
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "Parquet reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    } else {
        let csv_reader = CsvReader::from_path(file_path)
            .map_err(|e| ShellError::GenericError {
                error: "Error creating CSV reader".into(),
                msg: e.to_string(),
                span: Some(file_span),
                help: None,
                inner: vec![],
            })?
            .with_encoding(CsvEncoding::LossyUtf8);

        let csv_reader = match delimiter {
            None => csv_reader,
            Some(d) => {
                if d.item.len() != 1 {
                    return Err(ShellError::GenericError {
                        error: "Incorrect delimiter".into(),
                        msg: "Delimiter has to be one character".into(),
                        span: Some(d.span),
                        help: None,
                        inner: vec![],
                    });
                } else {
                    let delimiter = match d.item.chars().next() {
                        Some(d) => d as u8,
                        None => unreachable!(),
                    };
                    csv_reader.with_separator(delimiter)
                }
            }
        };

        let csv_reader = csv_reader.has_header(!no_header);

        let csv_reader = match maybe_schema {
            Some(schema) => csv_reader.with_schema(Some(schema.into())),
            None => csv_reader,
        };

        let csv_reader = match infer_schema {
            None => csv_reader,
            Some(r) => csv_reader.infer_schema(Some(r)),
        };

        let csv_reader = match skip_rows {
            None => csv_reader,
            Some(r) => csv_reader.with_skip_rows(r),
        };

        let csv_reader = match columns {
            None => csv_reader,
            Some(columns) => csv_reader.with_columns(Some(columns)),
        };

        let df: NuDataFrame = csv_reader
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "Parquet reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        df.cache_and_to_value(plugin, engine, call.head)
    }
}
