use crate::{
    dataframe::values::NuSchema,
    values::{CustomValueSupport, NuDataFrame, NuLazyFrame, PolarsFileType},
    EngineWrapper, PolarsPlugin,
};
use nu_path::expand_path_with;
use nu_utils::perf;

use nu_plugin::PluginCommand;
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

use std::{
    fs::File,
    io::BufReader,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use polars::{
    lazy::frame::LazyJsonLineReader,
    prelude::{
        CsvEncoding, IpcReader, JsonFormat, JsonReader, LazyCsvReader, LazyFileListReader,
        LazyFrame, ParquetReader, PlSmallStr, ScanArgsIpc, ScanArgsParquet, SerReader,
    },
};

use polars_io::{avro::AvroReader, csv::read::CsvReadOptions, HiveOptions};

const DEFAULT_INFER_SCHEMA: usize = 100;

#[derive(Clone)]
pub struct OpenDataFrame;

impl PluginCommand for OpenDataFrame {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars open"
    }

    fn description(&self) -> &str {
        "Opens CSV, JSON, NDJSON/JSON lines, arrow, avro, or parquet file to create dataframe. A lazy dataframe will be created by default, if supported."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "file",
                SyntaxShape::Filepath,
                "file path to load values from",
            )
            .switch("eager", "Open dataframe as an eager dataframe", None)
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
            .switch("truncate-ragged-lines", "Truncate lines that are longer than the schema. CSV file", None)
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
            PolarsFileType::Csv | PolarsFileType::Tsv => {
                from_csv(plugin, engine, call, &file_path, file_span)
            }
            PolarsFileType::Parquet => from_parquet(plugin, engine, call, &file_path, file_span),
            PolarsFileType::Arrow => from_arrow(plugin, engine, call, &file_path, file_span),
            PolarsFileType::Json => from_json(plugin, engine, call, &file_path, file_span),
            PolarsFileType::NdJson => from_ndjson(plugin, engine, call, &file_path, file_span),
            PolarsFileType::Avro => from_avro(plugin, engine, call, &file_path, file_span),
            _ => Err(PolarsFileType::build_unsupported_error(
                &ext,
                &[
                    PolarsFileType::Csv,
                    PolarsFileType::Tsv,
                    PolarsFileType::Parquet,
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
    if !call.has_flag("eager")? {
        let file: String = call.req(0)?;
        let args = ScanArgsParquet::default();
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

fn from_arrow(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    if !call.has_flag("eager")? {
        let file: String = call.req(0)?;
        let args = ScanArgsIpc {
            n_rows: None,
            cache: true,
            rechunk: false,
            row_index: None,
            cloud_options: None,
            include_file_paths: None,
            hive_options: HiveOptions::default(),
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

fn from_ndjson(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    file_path: &Path,
    file_span: Span,
) -> Result<Value, ShellError> {
    let infer_schema: NonZeroUsize = call
        .get_flag("infer-schema")?
        .and_then(NonZeroUsize::new)
        .unwrap_or(
            NonZeroUsize::new(DEFAULT_INFER_SCHEMA)
                .expect("The default infer-schema should be non zero"),
        );
    let maybe_schema = call
        .get_flag("schema")?
        .map(|schema| NuSchema::try_from(&schema))
        .transpose()?;

    if !call.has_flag("eager")? {
        let start_time = std::time::Instant::now();

        let df = LazyJsonLineReader::new(file_path)
            .with_infer_schema_length(Some(infer_schema))
            .with_schema(maybe_schema.map(|s| s.into()))
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: format!("NDJSON reader error: {e}"),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;

        perf!("Lazy NDJSON dataframe open", start_time, engine.use_color());

        let df = NuLazyFrame::new(false, df);
        df.cache_and_to_value(plugin, engine, call.head)
    } else {
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
            .infer_schema_len(Some(infer_schema));

        let reader = match maybe_schema {
            Some(schema) => reader.with_schema(schema.into()),
            None => reader,
        };

        let start_time = std::time::Instant::now();

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

        perf!(
            "Eager NDJSON dataframe open",
            start_time,
            engine.use_color()
        );

        df.cache_and_to_value(plugin, engine, call.head)
    }
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
    let infer_schema: usize = call
        .get_flag("infer-schema")?
        .unwrap_or(DEFAULT_INFER_SCHEMA);
    let skip_rows: Option<usize> = call.get_flag("skip-rows")?;
    let columns: Option<Vec<String>> = call.get_flag("columns")?;
    let maybe_schema = call
        .get_flag("schema")?
        .map(|schema| NuSchema::try_from(&schema))
        .transpose()?;
    let truncate_ragged_lines: bool = call.has_flag("truncate-ragged-lines")?;

    if !call.has_flag("eager")? {
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

        let csv_reader = csv_reader
            .with_has_header(!no_header)
            .with_infer_schema_length(Some(infer_schema))
            .with_schema(maybe_schema.map(Into::into))
            .with_truncate_ragged_lines(truncate_ragged_lines);

        let csv_reader = match skip_rows {
            None => csv_reader,
            Some(r) => csv_reader.with_skip_rows(r),
        };

        let start_time = std::time::Instant::now();
        let df: NuLazyFrame = csv_reader
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "CSV reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        perf!("Lazy CSV dataframe open", start_time, engine.use_color());

        df.cache_and_to_value(plugin, engine, call.head)
    } else {
        let start_time = std::time::Instant::now();
        let df = CsvReadOptions::default()
            .with_has_header(!no_header)
            .with_infer_schema_length(Some(infer_schema))
            .with_skip_rows(skip_rows.unwrap_or_default())
            .with_schema(maybe_schema.map(|s| s.into()))
            .with_columns(
                columns
                    .map(|v| v.iter().map(PlSmallStr::from).collect::<Vec<PlSmallStr>>())
                    .map(|v| Arc::from(v.into_boxed_slice())),
            )
            .map_parse_options(|options| {
                options
                    .with_separator(
                        delimiter
                            .as_ref()
                            .and_then(|d| d.item.chars().next().map(|c| c as u8))
                            .unwrap_or(b','),
                    )
                    .with_encoding(CsvEncoding::LossyUtf8)
                    .with_truncate_ragged_lines(truncate_ragged_lines)
            })
            .try_into_reader_with_file_path(Some(file_path.to_path_buf()))
            .map_err(|e| ShellError::GenericError {
                error: "Error creating CSV reader".into(),
                msg: e.to_string(),
                span: Some(file_span),
                help: None,
                inner: vec![],
            })?
            .finish()
            .map_err(|e| ShellError::GenericError {
                error: "CSV reader error".into(),
                msg: format!("{e:?}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;

        perf!("Eager CSV dataframe open", start_time, engine.use_color());

        let df = NuDataFrame::new(false, df);
        df.cache_and_to_value(plugin, engine, call.head)
    }
}
