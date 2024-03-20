use nu_protocol::ShellError;
use serde::{Deserialize, Serialize};

/// Protocol information, sent as a `Hello` message on initialization. This determines the
/// compatibility of the plugin and engine. They are considered to be compatible if the lower
/// version is semver compatible with the higher one.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolInfo {
    /// The name of the protocol being implemented. Only one protocol is supported. This field
    /// can be safely ignored, because not matching is a deserialization error
    pub protocol: Protocol,
    /// The semantic version of the protocol. This should be the version of the `nu-plugin`
    /// crate
    pub version: String,
    /// Supported optional features. This helps to maintain semver compatibility when adding new
    /// features
    pub features: Vec<Feature>,
}

impl Default for ProtocolInfo {
    fn default() -> ProtocolInfo {
        ProtocolInfo {
            protocol: Protocol::NuPlugin,
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec![],
        }
    }
}

impl ProtocolInfo {
    pub fn is_compatible_with(&self, other: &ProtocolInfo) -> Result<bool, ShellError> {
        fn parse_failed(error: semver::Error) -> ShellError {
            ShellError::PluginFailedToLoad {
                msg: format!("Failed to parse protocol version: {error}"),
            }
        }
        let mut versions = [
            semver::Version::parse(&self.version).map_err(parse_failed)?,
            semver::Version::parse(&other.version).map_err(parse_failed)?,
        ];

        versions.sort();

        // For example, if the lower version is 1.1.0, and the higher version is 1.2.3, the
        // requirement is that 1.2.3 matches ^1.1.0 (which it does)
        Ok(semver::Comparator {
            op: semver::Op::Caret,
            major: versions[0].major,
            minor: Some(versions[0].minor),
            patch: Some(versions[0].patch),
            pre: versions[0].pre.clone(),
        }
        .matches(&versions[1]))
    }
}

/// Indicates the protocol in use. Only one protocol is supported.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum Protocol {
    /// Serializes to the value `"nu-plugin"`
    #[serde(rename = "nu-plugin")]
    #[default]
    NuPlugin,
}

/// Indicates optional protocol features. This can help to make non-breaking-change additions to
/// the protocol. Features are not restricted to plain strings and can contain additional
/// configuration data.
///
/// Optional features should not be used by the protocol if they are not present in the
/// [`ProtocolInfo`] sent by the other side.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "name")]
pub enum Feature {
    /// A feature that was not recognized on deserialization. Attempting to serialize this feature
    /// is an error. Matching against it may only be used if necessary to determine whether
    /// unsupported features are present.
    #[serde(other, skip_serializing)]
    Unknown,
}
