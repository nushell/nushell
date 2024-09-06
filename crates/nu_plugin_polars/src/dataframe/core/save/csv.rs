use std::{fs::File, path::Path};

use nu_plugin::EvaluatedCall;
use nu_protocol::{ShellError, Span, Spanned};
use polars::prelude::{CsvWriter, SerWriter};
use polars_io::csv::write::{CsvWriterOptions, SerializeOptions};

use crate::values::{NuDataFrame, NuLazyFrame};

use super::polars_file_save_error;

pub(crate) fn command_lazy(
    call: &EvaluatedCall,
    lazy: &NuLazyFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
    let delimiter: Option<Spanned<String>> = call.get_flag("csv-delimiter")?;
    let separator = delimiter
        .and_then(|d| d.item.chars().next().map(|c| c as u8))
        .unwrap_or(b',');

    let no_header: bool = call.has_flag("csv-no-header")?;

    let options = CsvWriterOptions {
        include_header: !no_header,
        serialize_options: SerializeOptions {
            separator,
            ..SerializeOptions::default()
        },
        ..CsvWriterOptions::default()
    };

    lazy.to_polars()
        .sink_csv(file_path, options)
        .map_err(|e| polars_file_save_error(e, file_span))
}

pub(crate) fn command_eager(
    call: &EvaluatedCall,
    df: &NuDataFrame,
    file_path: &Path,
    file_span: Span,
) -> Result<(), ShellError> {
    let delimiter: Option<Spanned<String>> = call.get_flag("csv-delimiter")?;
    let no_header: bool = call.has_flag("csv-no-header")?;

    let mut file = File::create(file_path).map_err(|e| ShellError::GenericError {
        error: format!("Error with file name: {e}"),
        msg: "".into(),
        span: Some(file_span),
        help: None,
        inner: vec![],
    })?;

    let writer = CsvWriter::new(&mut file);

    let writer = if no_header {
        writer.include_header(false)
    } else {
        writer.include_header(true)
    };

    let mut writer = match delimiter {
        None => writer,
        Some(d) => {
            if d.item.len() != 1 {
                return Err(ShellError::GenericError {
                    error: "Incorrect delimiter".into(),
                    msg: "Delimiter has to be one char".into(),
                    span: Some(d.span),
                    help: None,
                    inner: vec![],
                });
            } else {
                let delimiter = match d.item.chars().next() {
                    Some(d) => d as u8,
                    None => unreachable!(),
                };

                writer.with_separator(delimiter)
            }
        }
    };

    writer
        .finish(&mut df.to_polars())
        .map_err(|e| ShellError::GenericError {
            error: format!("Error writing to file: {e}"),
            msg: e.to_string(),
            span: Some(file_span),
            help: None,
            inner: vec![],
        })?;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use crate::core::save::test::{test_eager_save, test_lazy_save};

    #[test]
    pub fn test_csv_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("csv")
    }

    #[test]
    pub fn test_csv_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("csv")
    }
}
