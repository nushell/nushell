use std::{fs::File, path::PathBuf};

use log::debug;
use nu_plugin::EvaluatedCall;
use nu_protocol::ShellError;
use polars::prelude::{ParquetWriteOptions, ParquetWriter, SinkOptions};

use crate::{
    command::core::resource::Resource,
    values::{NuDataFrame, NuLazyFrame},
};

use super::polars_file_save_error;

pub(crate) fn command_lazy(
    _call: &EvaluatedCall,
    lazy: &NuLazyFrame,
    resource: Resource,
) -> Result<(), ShellError> {
    let file_path = resource.as_string();
    let file_span = resource.span;
    debug!("Writing parquet file {file_path}");

    lazy.to_polars()
        .sink_parquet(
            resource.clone().into(),
            ParquetWriteOptions::default(),
            resource.cloud_options,
            SinkOptions::default(),
        )
        .and_then(|l| l.collect())
        .map_err(|e| polars_file_save_error(e, file_span))
        .map(|_| {
            debug!("Wrote parquet file {file_path}");
        })
}

pub(crate) fn command_eager(df: &NuDataFrame, resource: Resource) -> Result<(), ShellError> {
    let file_span = resource.span;
    let file_path: PathBuf = resource.try_into()?;
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

    use crate::command::core::save::test::{test_eager_save, test_lazy_save};

    #[test]
    pub fn test_parquet_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("parquet")
    }

    #[test]
    pub fn test_parquet_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("parquet")
    }
}
