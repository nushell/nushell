use std::error::Error;

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{Credentials, provider::ProvideCredentials};
use nu_protocol::ShellError;
use object_store::aws::AmazonS3ConfigKey;
use polars_io::cloud::CloudOptions;

use crate::PolarsPlugin;

async fn load_aws_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}

async fn aws_creds(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    if let Some(provider) = aws_config.credentials_provider() {
        Ok(Some(provider.provide_credentials().await.map_err(|e| {
            ShellError::GenericError {
                error: format!(
                    "Could not fetch AWS credentials: {} - {}",
                    e,
                    e.source().map(|e| format!("{e}")).unwrap_or("".to_string())
                ),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }
        })?))
    } else {
        Ok(None)
    }
}

async fn build_aws_cloud_configs() -> Result<Vec<(AmazonS3ConfigKey, String)>, ShellError> {
    let sdk_config = load_aws_config().await;
    let creds = aws_creds(&sdk_config)
        .await?
        .ok_or(ShellError::GenericError {
            error: "Could not determine AWS credentials".into(),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })?;

    let mut configs = vec![
        (AmazonS3ConfigKey::AccessKeyId, creds.access_key_id().into()),
        (
            AmazonS3ConfigKey::SecretAccessKey,
            creds.secret_access_key().into(),
        ),
    ];

    if let Some(token) = creds.session_token() {
        configs.push((AmazonS3ConfigKey::Token, token.into()))
    }

    Ok(configs)
}

pub(crate) fn build_cloud_options(plugin: &PolarsPlugin) -> Result<CloudOptions, ShellError> {
    let configs = plugin.runtime.block_on(build_aws_cloud_configs())?;
    Ok(CloudOptions::default().with_aws(configs))
}
