use object_store::gcp::GoogleCloudStorageBuilder;
use polars_io::cloud::{CloudOptions, GoogleConfigKey};

struct GoogleOptionsBuilder {
    builder: GoogleCloudStorageBuilder,
}

impl GoogleOptionsBuilder {
    fn new() -> Self {
        Self {
            builder: GoogleCloudStorageBuilder::new(),
        }
    }

    fn get_config_value(&self, key: GoogleConfigKey) -> Option<(GoogleConfigKey, String)> {
        self.builder.get_config_value(&key).map(|v| (key, v))
    }
}

pub(crate) fn build_cloud_options() -> Result<CloudOptions, nu_protocol::ShellError> {
    let builder = GoogleOptionsBuilder::new();
    // GOOGLE_SERVICE_ACCOUNT: location of service account file
    // GOOGLE_SERVICE_ACCOUNT_PATH: (alias) location of service account file
    // SERVICE_ACCOUNT: (alias) location of service account file
    // GOOGLE_SERVICE_ACCOUNT_KEY: JSON serialized service account key
    // GOOGLE_BUCKET: bucket name
    // GOOGLE_BUCKET_NAME: (alias) bucket name
    let configs = vec![
        GoogleConfigKey::ServiceAccount,
        GoogleConfigKey::ServiceAccountKey,
        GoogleConfigKey::Bucket,
    ]
    .into_iter()
    .filter_map(|key| builder.get_config_value(key))
    .collect::<Vec<(GoogleConfigKey, String)>>();

    Ok(CloudOptions::default().with_gcp(configs))
}
