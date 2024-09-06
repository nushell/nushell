use std::{fs::File, path::Path};

use nu_plugin::EvaluatedCall;
use nu_protocol::{ShellError, Span};
use polars::prelude::ParquetWriter;
use polars_io::parquet::write::ParquetWriteOptions;

use crate::values::{NuDataFrame, NuLazyFrame};

use super::polars_file_save_error;

pub(crate) fn command_lazy(
    _call: &EvaluatedCall,
    lazy: &NuLazyFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
    lazy.to_polars()
        .sink_parquet(file_path, ParquetWriteOptions::default())
        .map_err(|e| polars_file_save_error(e, file_span))
}

pub(crate) fn command_eager(
    df: &NuDataFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
    let file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: "Error with file name".into(),
        msg: e.to_string(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;
    let mut polars_df = df.to_polars();
    ParquetWriter::new(file)
        .finish(&mut polars_df)
        .map_err(|e| ShellError::GenericError {
            error: "Error saving file".into(),
            msg: e.to_string(),
            span: Some(file_span),
            help: None,
            inner: vec![],
        })?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod test {

    use crate::core::save::test::{test_eager_save, test_lazy_save};

    #[test]
    pub fn test_parquet_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("parquet")
    }

    #[test]
    pub fn test_parquet_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("parquet")
    }
}
