use nu_protocol::ShellError;
use polars::prelude::PlPath;
use polars_io::cloud::{CloudOptions, CloudType};

use crate::PolarsPlugin;

mod aws;
mod azure;

pub(crate) fn build_cloud_options(
    plugin: &PolarsPlugin,
    path: &PlPath,
) -> Result<Option<CloudOptions>, ShellError> {
    match path
        .cloud_scheme()
        .map(|ref scheme| CloudType::from_cloud_scheme(scheme))
    {
        Some(CloudType::Aws) => aws::build_cloud_options(plugin).map(Some),
        Some(CloudType::Azure) => azure::build_cloud_options().map(Some),

        _ => Ok(None),
    }
}
