use nu_protocol::ShellError;
use polars_io::cloud::CloudOptions;

use crate::PolarsPlugin;

mod aws;

enum CloudType {
    Aws,
}

fn determine_cloud_type(path: &str) -> Option<CloudType> {
    if path.starts_with("s3://") | path.starts_with("s3a://") {
        Some(CloudType::Aws)
    } else {
        None
    }
}

/// Returns true if it is a supported cloud url
pub(crate) fn is_cloud_url(path: &str) ->bool {
    determine_cloud_type(path).is_some()
}

pub(crate) fn build_cloud_options(
    plugin: &PolarsPlugin,
    path: &str,
) -> Result<Option<CloudOptions>, ShellError> {
    match determine_cloud_type(path) {
        Some(CloudType::Aws) => aws::build_cloud_options(plugin).map(|c| Some(c)),
        _ => Ok(None),
    }
}
