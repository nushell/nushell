use aws_config::BehaviorVersion;
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use aws_types::SdkConfig;
use futures::executor::block_on;
use nu_protocol::ShellError;
use polars_io::cloud::{AmazonS3ConfigKey, CloudOptions};

pub(crate) fn build_aws_cloud_options() -> Result<Option<CloudOptions>, ShellError> {
    let aws_config = load_config();
    let mut configs: Vec<(AmazonS3ConfigKey, String)> = Vec::with_capacity(3);
    if let Some(region) = aws_config.region() {
        configs.push((AmazonS3ConfigKey::Region, region.to_string()));
    }
    if let Some(credentials) = accces_key_id(&aws_config)? {
        configs.push((
            AmazonS3ConfigKey::AccessKeyId,
            credentials.access_key_id().to_string(),
        ));
        configs.push((
            AmazonS3ConfigKey::SecretAccessKey,
            credentials.secret_access_key().to_string(),
        ));
    }

    Ok(Some(CloudOptions::default().with_aws(configs)))
}

fn load_config() -> SdkConfig {
    block_on(aws_config::load_defaults(BehaviorVersion::latest()))
}

fn accces_key_id(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    aws_config
        .credentials_provider()
        .map(|provider| {
            block_on(provider.provide_credentials()).map_err(|e| ShellError::GenericError {
                error: format!("Could not fetch AWS credentials: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        })
        .transpose()
}
