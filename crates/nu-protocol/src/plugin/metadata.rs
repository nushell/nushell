use serde::{Deserialize, Serialize};

/// Metadata about the installed plugin. This is cached in the registry file along with the
/// signatures. None of the metadata fields are required, and more may be added in the future.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[non_exhaustive]
pub struct PluginMetadata {
    /// The version of the plugin itself, as self-reported.
    pub version: Option<String>,
}

impl PluginMetadata {
    /// Create empty metadata.
    pub const fn new() -> PluginMetadata {
        PluginMetadata { version: None }
    }

    /// Set the version of the plugin on the metadata. A suggested way to construct this is:
    ///
    /// ```no_run
    /// # use nu_protocol::PluginMetadata;
    /// # fn example() -> PluginMetadata {
    /// PluginMetadata::new().with_version(env!("CARGO_PKG_VERSION"))
    /// # }
    /// ```
    ///
    /// which will use the version of your plugin's crate from its `Cargo.toml` file.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

impl Default for PluginMetadata {
    fn default() -> Self {
        Self::new()
    }
}
