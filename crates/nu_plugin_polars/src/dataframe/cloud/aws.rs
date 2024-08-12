use polars_io::cloud::{AmazonS3ConfigKey, CloudOptions};

pub(crate) fn build_aws_cloud_options() -> CloudOptions {
    CloudOptions::default().with_aws(Vec::<(AmazonS3ConfigKey, String)>::new())
}
