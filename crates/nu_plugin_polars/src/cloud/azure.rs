use std::str::FromStr;

// Use the re-export from polars_io so config keys match the object_store version
// that polars depends on (avoids dual object_store version type mismatches).
use polars_io::cloud::{AzureConfigKey, CloudOptions};

/// Collect Azure config from environment variables.
///
/// Mirrors `object_store::azure::MicrosoftAzureBuilder::from_env`:
/// * `AZURE_STORAGE_ACCOUNT_NAME`: storage account name
/// * `AZURE_STORAGE_ACCOUNT_KEY`: storage account master key
/// * `AZURE_STORAGE_ACCESS_KEY`: alias for `AZURE_STORAGE_ACCOUNT_KEY`
/// * `AZURE_STORAGE_CLIENT_ID` → client id for service principal authorization
/// * `AZURE_STORAGE_CLIENT_SECRET` → client secret for service principal authorization
/// * `AZURE_STORAGE_TENANT_ID` → tenant id used in oauth flows
fn azure_configs_from_env() -> Vec<(AzureConfigKey, String)> {
    std::env::vars()
        .filter_map(|(key, value)| {
            if key.starts_with("AZURE_") {
                AzureConfigKey::from_str(&key.to_ascii_lowercase())
                    .ok()
                    .map(|config_key| (config_key, value))
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn build_cloud_options() -> Result<CloudOptions, nu_protocol::ShellError> {
    Ok(CloudOptions::default().with_azure(azure_configs_from_env()))
}
