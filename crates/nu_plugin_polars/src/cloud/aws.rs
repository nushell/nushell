use std::error::Error;

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use nu_protocol::ShellError;
use polars_io::cloud::CloudOptions;

async fn aws_load_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}

async fn aws_creds(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    if let Some(provider) = aws_config.credentials_provider() {
        Ok(Some(provider.provide_credentials().await.map_err(|e| {
            ShellError::GenericError {
                error: format!(
                    "Could not fetch AWS credentials: {} - {}",
                    e,
                    e.source()
                        .map(|e| format!("{}", e))
                        .unwrap_or("".to_string())
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

async fn aws_cloud_options() -> CloudOptions {
    CloudOptions::
}
