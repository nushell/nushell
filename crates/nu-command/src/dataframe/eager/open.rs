use super::super::values::{NuDataFrame, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};

use std::{fs::File, io::BufReader, path::PathBuf};

use polars::prelude::{
    CsvEncoding, CsvReader, IpcReader, JsonReader, LazyCsvReader, LazyFrame, ParallelStrategy,
    ParquetReader, ScanArgsIpc, ScanArgsParquet, SerReader,
};

#[derive(Clone)]
pub struct OpenDataFrame;

impl Command for OpenDataFrame {
    fn name(&self) -> &str {
        "open-df"
    }

    fn usage(&self) -> &str {
        "Opens csv, json, arrow, or parquet file to create dataframe"
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
                "File type: csv, tsv, json, parquet, arrow. If omitted, derive from file extension",
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
            .input_type(Type::Any)
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a file name and creates a dataframe",
            example: "open test.csv",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let file: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;

    let type_option: Option<Spanned<String>> = call.get_flag(engine_state, stack, "type")?;

    let type_id = match &type_option {
        Some(ref t) => Some((t.item.to_owned(), "Invalid type", t.span)),
        None => match file.item.extension() {
            Some(e) => Some((
                e.to_string_lossy().into_owned(),
                "Invalid extension",
                file.span,
            )),
            None => None,
        },
    };

    match type_id {
        Some((e, msg, blamed)) => match e.as_str() {
            "csv" | "tsv" => from_csv(engine_state, stack, call),
            "parquet" => from_parquet(engine_state, stack, call),
            "ipc" | "arrow" => from_ipc(engine_state, stack, call),
            "json" => from_json(engine_state, stack, call),
            _ => Err(ShellError::FileNotFoundCustom(
                format!(
                    "{}. Supported values: csv, tsv, parquet, ipc, arrow, json",
                    msg
                ),
                blamed,
            )),
        },
        None => Err(ShellError::FileNotFoundCustom(
            "File without extension".into(),
            file.span,
        )),
    }
    .map(|value| PipelineData::Value(value, None))
}

fn from_parquet(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Value, ShellError> {
    if call.has_flag("lazy") {
        let file: String = call.req(engine_state, stack, 0)?;
        let args = ScanArgsParquet {
            n_rows: None,
            cache: true,
            parallel: ParallelStrategy::Auto,
            rechunk: false,
            row_count: None,
            low_memory: false,
        };

        let df: NuLazyFrame = LazyFrame::scan_parquet(file, args)
            .map_err(|e| {
                ShellError::GenericError(
                    "Parquet reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        df.into_value(call.head)
    } else {
        let file: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;
        let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;

        let r = File::open(&file.item).map_err(|e| {
            ShellError::GenericError(
                "Error opening file".into(),
                e.to_string(),
                Some(file.span),
                None,
                Vec::new(),
            )
        })?;
        let reader = ParquetReader::new(r);

        let reader = match columns {
            None => reader,
            Some(columns) => reader.with_columns(Some(columns)),
        };

        let df: NuDataFrame = reader
            .finish()
            .map_err(|e| {
                ShellError::GenericError(
                    "Parquet reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        Ok(df.into_value(call.head))
    }
}

fn from_ipc(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Value, ShellError> {
    if call.has_flag("lazy") {
        let file: String = call.req(engine_state, stack, 0)?;
        let args = ScanArgsIpc {
            n_rows: None,
            cache: true,
            rechunk: false,
            row_count: None,
            memmap: true,
        };

        let df: NuLazyFrame = LazyFrame::scan_ipc(file, args)
            .map_err(|e| {
                ShellError::GenericError(
                    "IPC reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        df.into_value(call.head)
    } else {
        let file: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;
        let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;

        let r = File::open(&file.item).map_err(|e| {
            ShellError::GenericError(
                "Error opening file".into(),
                e.to_string(),
                Some(file.span),
                None,
                Vec::new(),
            )
        })?;
        let reader = IpcReader::new(r);

        let reader = match columns {
            None => reader,
            Some(columns) => reader.with_columns(Some(columns)),
        };

        let df: NuDataFrame = reader
            .finish()
            .map_err(|e| {
                ShellError::GenericError(
                    "IPC reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        Ok(df.into_value(call.head))
    }
}

fn from_json(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Value, ShellError> {
    let file: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;
    let file = File::open(&file.item).map_err(|e| {
        ShellError::GenericError(
            "Error opening file".into(),
            e.to_string(),
            Some(file.span),
            None,
            Vec::new(),
        )
    })?;

    let buf_reader = BufReader::new(file);
    let reader = JsonReader::new(buf_reader);

    let df: NuDataFrame = reader
        .finish()
        .map_err(|e| {
            ShellError::GenericError(
                "Json reader error".into(),
                format!("{:?}", e),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?
        .into();

    Ok(df.into_value(call.head))
}

fn from_csv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Value, ShellError> {
    let delimiter: Option<Spanned<String>> = call.get_flag(engine_state, stack, "delimiter")?;
    let no_header: bool = call.has_flag("no-header");
    let infer_schema: Option<usize> = call.get_flag(engine_state, stack, "infer-schema")?;
    let skip_rows: Option<usize> = call.get_flag(engine_state, stack, "skip-rows")?;
    let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;

    if call.has_flag("lazy") {
        let file: String = call.req(engine_state, stack, 0)?;
        let csv_reader = LazyCsvReader::new(file);

        let csv_reader = match delimiter {
            None => csv_reader,
            Some(d) => {
                if d.item.len() != 1 {
                    return Err(ShellError::GenericError(
                        "Incorrect delimiter".into(),
                        "Delimiter has to be one character".into(),
                        Some(d.span),
                        None,
                        Vec::new(),
                    ));
                } else {
                    let delimiter = match d.item.chars().next() {
                        Some(d) => d as u8,
                        None => unreachable!(),
                    };
                    csv_reader.with_delimiter(delimiter)
                }
            }
        };

        let csv_reader = csv_reader.has_header(!no_header);

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
            .map_err(|e| {
                ShellError::GenericError(
                    "Parquet reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        df.into_value(call.head)
    } else {
        let file: Spanned<PathBuf> = call.req(engine_state, stack, 0)?;
        let csv_reader = CsvReader::from_path(&file.item)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error creating CSV reader".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?
            .with_encoding(CsvEncoding::LossyUtf8);

        let csv_reader = match delimiter {
            None => csv_reader,
            Some(d) => {
                if d.item.len() != 1 {
                    return Err(ShellError::GenericError(
                        "Incorrect delimiter".into(),
                        "Delimiter has to be one character".into(),
                        Some(d.span),
                        None,
                        Vec::new(),
                    ));
                } else {
                    let delimiter = match d.item.chars().next() {
                        Some(d) => d as u8,
                        None => unreachable!(),
                    };
                    csv_reader.with_delimiter(delimiter)
                }
            }
        };

        let csv_reader = csv_reader.has_header(!no_header);

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
            .map_err(|e| {
                ShellError::GenericError(
                    "Parquet reader error".into(),
                    format!("{:?}", e),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        Ok(df.into_value(call.head))
    }
}
