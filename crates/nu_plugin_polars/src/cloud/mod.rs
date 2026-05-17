use nu_protocol::ShellError;
use polars::prelude::PlRefPath;
use polars_io::cloud::{CloudOptions, CloudType};

use crate::PolarsPlugin;

mod aws;
mod azure;
mod gcp;

pub(crate) fn build_cloud_options(
    plugin: &PolarsPlugin,
    path: &PlRefPath,
) -> Result<Option<CloudOptions>, ShellError> {
    match path.scheme().map(CloudType::from_cloud_scheme) {
        Some(CloudType::Aws) => aws::build_cloud_options(plugin).map(Some),
        Some(CloudType::Azure) => azure::build_cloud_options().map(Some),
        Some(CloudType::Gcp) => gcp::build_cloud_options().map(Some),

        _ => Ok(None),
    }
}
