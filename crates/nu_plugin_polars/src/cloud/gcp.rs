use std::str::FromStr;

// Use the re-export from polars_io so config keys match the object_store version
// that polars depends on (avoids dual object_store version type mismatches).
use polars_io::cloud::{CloudOptions, GoogleConfigKey};

/// Collect GCP config from environment variables.
///
/// Mirrors `object_store::gcp::GoogleCloudStorageBuilder::from_env`:
/// * `GOOGLE_SERVICE_ACCOUNT`: location of service account file
/// * `GOOGLE_SERVICE_ACCOUNT_PATH`: (alias) location of service account file
/// * `SERVICE_ACCOUNT`: (alias) location of service account file
/// * `GOOGLE_SERVICE_ACCOUNT_KEY`: JSON serialized service account key
/// * `GOOGLE_BUCKET`: bucket name
/// * `GOOGLE_BUCKET_NAME`: (alias) bucket name
fn gcp_configs_from_env() -> Vec<(GoogleConfigKey, String)> {
    let mut configs = Vec::new();

    if let Ok(service_account_path) = std::env::var("SERVICE_ACCOUNT") {
        configs.push((GoogleConfigKey::ServiceAccount, service_account_path));
    }

    for (key, value) in std::env::vars() {
        if key.starts_with("GOOGLE_")
            && let Ok(config_key) = GoogleConfigKey::from_str(&key.to_ascii_lowercase())
        {
            // Later GOOGLE_* values override earlier entries (including SERVICE_ACCOUNT).
            if let Some(existing) = configs.iter_mut().find(|(k, _)| *k == config_key) {
                existing.1 = value;
            } else {
                configs.push((config_key, value));
            }
        }
    }

    configs
}

pub(crate) fn build_cloud_options() -> Result<CloudOptions, nu_protocol::ShellError> {
    Ok(CloudOptions::default().with_gcp(gcp_configs_from_env()))
}
