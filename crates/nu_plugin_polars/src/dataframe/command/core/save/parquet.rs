use std::fs::File;

use log::debug;
use nu_plugin::EvaluatedCall;
use nu_protocol::ShellError;
use polars::prelude::{ParquetWriter, SinkOptions};
use polars_io::parquet::write::ParquetWriteOptions;

use crate::{
    command::core::{resource::Resource, save::sink_target_from_string},
    values::{NuDataFrame, NuLazyFrame},
};

use super::polars_file_save_error;

pub(crate) fn command_lazy(
    _call: &EvaluatedCall,
    lazy: &NuLazyFrame,
    resource: Resource,
) -> Result<(), ShellError> {
    let file_path = sink_target_from_string(resource.path.clone());
    let file_span = resource.span;
    debug!("Writing parquet file {}", resource.path);

    lazy.to_polars()
        .sink_parquet(
            file_path,
            ParquetWriteOptions::default(),
            resource.cloud_options,
            SinkOptions::default(),
        )
        .and_then(|l| l.collect())
        .map_err(|e| polars_file_save_error(e, file_span))
        .map(|_| {
            debug!("Wrote parquet file {}", resource.path);
        })
}

pub(crate) fn command_eager(df: &NuDataFrame, resource: Resource) -> Result<(), ShellError> {
    let file_path = resource.path;
    let file_span = resource.span;
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
