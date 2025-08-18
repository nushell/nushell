use std::fs::File;

use nu_plugin::EvaluatedCall;
use nu_protocol::ShellError;
use nu_utils::uformat;
use polars_io::SerWriter;
use polars_io::avro::{AvroCompression, AvroWriter};

use crate::command::core::resource::Resource;
use crate::values::NuDataFrame;

fn get_compression(call: &EvaluatedCall) -> Result<Option<AvroCompression>, ShellError> {
    if let Some((compression, span)) = call
        .get_flag_value("avro-compression")
        .map(|e| e.as_str().map(|s| (s.to_owned(), e.span())))
        .transpose()?
    {
        match compression.as_ref() {
            "snappy" => Ok(Some(AvroCompression::Snappy)),
            "deflate" => Ok(Some(AvroCompression::Deflate)),
            _ => Err(ShellError::IncorrectValue {
                msg: "compression must be one of deflate or snappy".into(),
                val_span: span,
                call_span: span,
            }),
        }
    } else {
        Ok(None)
    }
}

pub(crate) fn command_eager(
    call: &EvaluatedCall,
    df: &NuDataFrame,
    resource: Resource,
) -> Result<(), ShellError> {
    let file_path = resource.path;
    let file_span = resource.span;
    let compression = get_compression(call)?;

    let file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: uformat!("Error with file name: {e}"),
        msg: "".into(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;

    AvroWriter::new(file)
        .with_compression(compression)
        .finish(&mut df.to_polars())
        .map_err(|e| ShellError::GenericError {
            error: "Error saving file".into(),
            msg: e.to_string().into(),
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
    pub fn test_avro_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("avro")
    }

    #[test]
    pub fn test_avro_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("avro")
    }
}
