use nu_protocol::ShellError;
use polars_io::cloud::CloudOptions;
use url::Url;

use crate::PolarsPlugin;

mod aws;

enum CloudType {
    Aws,
}

fn determine_cloud_type(url: &Url) -> Option<CloudType> {
    match url.scheme() {
        "s3" | "s3a" => Some(CloudType::Aws),
        _ => None,
    }
}

pub(crate) fn build_cloud_options(
    plugin: &PolarsPlugin,
    url: &Url,
) -> Result<Option<CloudOptions>, ShellError> {
    match determine_cloud_type(url) {
        Some(CloudType::Aws) => aws::build_cloud_options(plugin).map(Some),
        _ => Ok(None),
    }
}
