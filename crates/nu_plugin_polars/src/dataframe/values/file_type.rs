enum PolarsFileType {
    Csv,
    Parquet,
    Arrow,
    Json,
    NdJson,
    Excel,
}

impl TryFrom<&str> for PolarsFileType {
    type Error = ShellError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "csv" => Ok(PolarsFileType::Csv),
            "parquet" => Ok(PolarsFileType::Parquet),
            "ipc" | "arrow" => Ok(PolarsFileType::Arrow),
            "json" => Ok(PolarsFileType::Json),
            "jsonl" | "ndjson" => Ok(PolarsFileType::NdJson),
            "excel" => Ok(PolarsFileType::Excel),
            _ => Ok(PolarsFileType::None),
        }
    }
}
