use nu_protocol::ShellError;
use polars::prelude::PlPath;
use polars_io::cloud::CloudOptions;
use polars_utils::plpath::CloudScheme;

use crate::PolarsPlugin;

mod aws;

enum CloudType {
    Aws,
}

fn determine_cloud_type(plpath: &PlPath) -> Option<CloudType> {
    match plpath.cloud_scheme() {
        Some(CloudScheme::S3) | Some(CloudScheme::S3a) => Some(CloudType::Aws),
        _ => None,
    }
}

pub(crate) fn build_cloud_options(
    plugin: &PolarsPlugin,
    path: &PlPath,
) -> Result<Option<CloudOptions>, ShellError> {
    match determine_cloud_type(path) {
        Some(CloudType::Aws) => aws::build_cloud_options(plugin).map(Some),
        _ => Ok(None),
    }
}
