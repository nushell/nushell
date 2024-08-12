use nu_path::Path;
use nu_protocol::ShellError;
use polars_io::cloud::CloudOptions;

mod aws;

pub fn cloud_options_from_path(path: &Path) -> Result<Option<CloudOptions>, ShellError> {
    if path.starts_with("s3://") {
        aws::build_aws_cloud_options()
    } else {
        Ok(None)
    }
}
