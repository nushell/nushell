use std::{fs::File, io::BufWriter, path::Path};

use nu_plugin::EvaluatedCall;
use nu_protocol::{ShellError, Span};
use polars::prelude::{JsonWriter, SerWriter};
use polars_io::json::JsonWriterOptions;

use crate::values::{NuDataFrame, NuLazyFrame};

use super::polars_file_save_error;

pub(crate) fn command_lazy(
    _call: &EvaluatedCall,
    lazy: &NuLazyFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
    lazy.to_polars()
        // todo - add cloud options
        .sink_json(file_path, JsonWriterOptions::default(), None)
        .map_err(|e| polars_file_save_error(e, file_span))
}

pub(crate) fn command_eager(
    df: &NuDataFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
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
    pub fn test_arrow_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("ndjson")
    }

    #[test]
    pub fn test_arrow_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("ndjson")
    }
}
