use std::path::Path;

use polars_io::cloud::CloudOptions;

mod aws;

pub fn cloud_options_from_path(path: &Path) -> Option<CloudOptions> {
    if path.starts_with("s3://") {
        Some(aws::build_aws_cloud_options())
    } else {
        None
    }
}
