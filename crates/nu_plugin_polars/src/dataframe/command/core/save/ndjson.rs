use std::{fs::File, io::BufWriter, path::PathBuf};

use log::debug;
use nu_plugin::EvaluatedCall;
use nu_protocol::ShellError;
use polars::prelude::{JsonWriter, JsonWriterOptions, SerWriter, SinkOptions};

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
    debug!("Writing ndjson file {file_path}");
    lazy.to_polars()
        .sink_json(
            resource.clone().into(),
            JsonWriterOptions::default(),
            resource.cloud_options,
            SinkOptions::default(),
        )
        .and_then(|l| l.collect())
        .map_err(|e| polars_file_save_error(e, file_span))
        .map(|_| {
            debug!("Wrote ndjson file {file_path}");
        })
}

pub(crate) fn command_eager(df: &NuDataFrame, resource: Resource) -> Result<(), ShellError> {
    let file_span = resource.span;
    let file_path: PathBuf = resource.try_into()?;
    let file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: format!("Error with file name: {e}"),
        msg: "".into(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;
    let buf_writer = BufWriter::new(file);

    JsonWriter::new(buf_writer)
        .finish(&mut df.to_polars())
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
pub mod test {
    use crate::command::core::save::test::{test_eager_save, test_lazy_save};

    #[test]
    pub fn test_ndjson_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("ndjson")
    }

    #[test]
    pub fn test_ndjson_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("ndjson")
    }
}
