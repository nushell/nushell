use std::{fs::File, sync::Arc};

use log::debug;
use nu_plugin::EvaluatedCall;
use nu_protocol::{ShellError, shell_error::generic::GenericError};
use polars::prelude::{FileWriteFormat, IpcWriter, IpcWriterOptions, SerWriter, UnifiedSinkArgs};

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
    debug!("Writing ipc file {file_path}");
    lazy.to_polars()
        .sink(
            resource.clone().into(),
            FileWriteFormat::Ipc(IpcWriterOptions::default()),
            UnifiedSinkArgs {
                cloud_options: resource.cloud_options.map(Arc::new),
                ..Default::default()
            },
        )
        .and_then(|l| l.collect())
        .map(|_| {
            debug!("Wrote ipc file {file_path}");
        })
        .map_err(|e| polars_file_save_error(e, file_span))
}

pub(crate) fn command_eager(df: &NuDataFrame, resource: Resource) -> Result<(), ShellError> {
    let file_span = resource.span;
    let file_path = resource.as_path_buf();
    let mut file = File::create(file_path).map_err(|e| {
        ShellError::Generic(GenericError::new(
            format!("Error with file name: {e}"),
            "",
            file_span,
        ))
    })?;

    IpcWriter::new(&mut file)
        .finish(&mut df.to_polars())
        .map_err(|e| {
            ShellError::Generic(GenericError::new(
                "Error saving file",
                e.to_string(),
                file_span,
            ))
        })?;
    Ok(())
}

#[cfg(test)]
pub mod test {

    use crate::command::core::save::test::{test_eager_save, test_lazy_save};

    #[test]
    pub fn test_arrow_eager_save() -> Result<(), Box<dyn std::error::Error>> {
        test_eager_save("arrow")
    }

    #[test]
    pub fn test_arrow_lazy_save() -> Result<(), Box<dyn std::error::Error>> {
        test_lazy_save("arrow")
    }
}
