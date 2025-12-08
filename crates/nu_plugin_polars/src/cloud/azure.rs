use object_store::azure::MicrosoftAzureBuilder;
use polars_io::cloud::{AzureConfigKey, CloudOptions};

struct AzureOptionsBuilder {
    builder: MicrosoftAzureBuilder,
}

impl AzureOptionsBuilder {
    fn new() -> Self {
        Self {
            builder: MicrosoftAzureBuilder::new(),
        }
    }

    fn get_config_value(&self, key: AzureConfigKey) -> Option<(AzureConfigKey, String)> {
        self.builder.get_config_value(&key).map(|v| (key, v))
    }
}

pub(crate) fn build_cloud_options() -> Result<CloudOptions, nu_protocol::ShellError> {
    let builder = AzureOptionsBuilder::new();
    // Variables extracted from environment:
    // * AZURE_STORAGE_ACCOUNT_NAME: storage account name
    // * AZURE_STORAGE_ACCOUNT_KEY: storage account master key
    // * AZURE_STORAGE_ACCESS_KEY: alias for AZURE_STORAGE_ACCOUNT_KEY
    // * AZURE_STORAGE_CLIENT_ID -> client id for service principal authorization
    // * AZURE_STORAGE_CLIENT_SECRET -> client secret for service principal authorization
    // * AZURE_STORAGE_TENANT_ID -> tenant id used in oauth flows
    let configs = vec![
        AzureConfigKey::AccountName,
        AzureConfigKey::AccessKey,
        AzureConfigKey::AccessKey,
        AzureConfigKey::ClientId,
        AzureConfigKey::ClientSecret,
        AzureConfigKey::AuthorityId,
    ]
    .into_iter()
    .filter_map(|key| builder.get_config_value(key))
    .collect::<Vec<(AzureConfigKey, String)>>();

    Ok(CloudOptions::default().with_azure(configs))
}
