use nu_protocol::{ShellError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum PolarsFileType {
    Csv,
    Tsv,
    Parquet,
    Arrow,
    Json,
    Avro,
    NdJson,
    Unknown,
}

impl PolarsFileType {
    pub fn build_unsupported_error(
        extension: &str,
        supported_types: &[PolarsFileType],
        span: Span,
    ) -> ShellError {
        let type_string = supported_types
            .iter()
            .map(|ft| ft.to_str())
            .collect::<Vec<&'static str>>()
            .join(", ");

        ShellError::GenericError {
            error: format!("Unsupported type {extension} expected {type_string}"),
            msg: "".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            PolarsFileType::Csv => "csv",
            PolarsFileType::Tsv => "tsv",
            PolarsFileType::Parquet => "parquet",
            PolarsFileType::Arrow => "arrow",
            PolarsFileType::Json => "json",
            PolarsFileType::Avro => "avro",
            PolarsFileType::NdJson => "ndjson",
            PolarsFileType::Unknown => "unknown",
        }
    }
}

impl From<&str> for PolarsFileType {
    fn from(file_type: &str) -> Self {
        match file_type {
            "csv" => PolarsFileType::Csv,
            "tsv" => PolarsFileType::Tsv,
            "parquet" | "parq" | "pq" => PolarsFileType::Parquet,
            "ipc" | "arrow" => PolarsFileType::Arrow,
            "json" => PolarsFileType::Json,
            "avro" => PolarsFileType::Avro,
            "jsonl" | "ndjson" => PolarsFileType::NdJson,
            _ => PolarsFileType::Unknown,
        }
    }
}
