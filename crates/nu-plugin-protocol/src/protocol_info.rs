use nu_protocol::ShellError;
use semver::Prerelease;

/// Independent version of the plugin **wire** protocol (not the Nushell release version).
///
/// This is the compatibility contract exchanged in the `Hello` handshake. It does **not** track
/// `CARGO_PKG_VERSION` / the workspace package version. Only bump this when the plugin protocol
/// itself changes in a way that affects engine/plugin compatibility.
///
/// # Versioning policy (0.x)
///
/// The protocol stays on a `0.x` line while the contract is still evolving (top-level messages use
/// explicit serde, but many nested engine types still serialize via derive). Do not jump to `1.0.0`
/// until the wire contract is intentionally stable.
///
/// Handshake compatibility uses a semver **caret** against the lower of the two versions. On `0.x`
/// that means only the same minor line is compatible (`^0.1.0` ⇒ `>=0.1.0, <0.2.0`):
///
/// | Change | Bump |
/// | --- | --- |
/// | Breaking wire change (rename/remove field/variant, change meaning) | **minor** (`0.1` → `0.2`) |
/// | Additive change older peers must not see without negotiation | **minor** (or Feature-gated) |
/// | Compatible fix that preserves accepted messages | **patch** (`0.1.0` → `0.1.1`) |
/// | Documentation only | none |
///
/// # Nested types still coupled to the engine
///
/// Explicit serde on `PluginInput` / `PluginOutput` does **not** fully freeze nested payloads such
/// as `Value` subfields beyond the explicit `Value` mapping, `ShellError`, `Config`, `PluginSignature`,
/// `ast::Call`, `IrBlock`, and similar. Prefer wire **serialization snapshots** as the CI guardrail;
/// bump this constant when those nested shapes change on the wire in a compatibility-sensitive way.
///
/// `0.1.0` is the first freeze of the **current** wire surface under separate protocol versioning
/// (not a claim that the wire still matches historical Nushell 0.93).
pub const PLUGIN_PROTOCOL_VERSION: &str = "0.1.0";

/// Protocol information, sent as a `Hello` message on initialization. This determines the
/// compatibility of the plugin and engine. They are considered to be compatible if the lower
/// version is semver compatible with the higher one.
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    /// The name of the protocol being implemented. Only one protocol is supported. This field
    /// can be safely ignored, because not matching is a deserialization error
    pub protocol: Protocol,
    /// The semantic version of the plugin wire protocol ([`PLUGIN_PROTOCOL_VERSION`]), not the
    /// Nushell package / crate version.
    pub version: String,
    /// Supported optional features. This helps to maintain semver compatibility when adding new
    /// features
    pub features: Vec<Feature>,
}

impl Default for ProtocolInfo {
    fn default() -> ProtocolInfo {
        ProtocolInfo {
            protocol: Protocol::NuPlugin,
            version: PLUGIN_PROTOCOL_VERSION.into(),
            features: default_features(),
        }
    }
}

impl ProtocolInfo {
    /// True if the version specified in `self` is compatible with the version specified in `other`.
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

        // The version may carry a prerelease label (e.g. 0.1.0-nightly.1). Clear it so pre-release
        // builds remain compatible with the matching base protocol version.
        versions[1].pre = Prerelease::EMPTY;
        versions[0].pre = Prerelease::EMPTY;

        // For example, if the lower version is 0.1.0, and the higher version is 0.1.3, the
        // requirement is that 0.1.3 matches ^0.1.0 (which it does). On 0.x, 0.2.0 does not match.
        Ok(semver::Comparator {
            op: semver::Op::Caret,
            major: versions[0].major,
            minor: Some(versions[0].minor),
            patch: Some(versions[0].patch),
            pre: versions[0].pre.clone(),
        }
        .matches(&versions[1]))
    }

    /// True if the protocol info contains a feature compatible with the given feature.
    pub fn supports_feature(&self, feature: &Feature) -> bool {
        self.features.iter().any(|f| feature.is_compatible_with(f))
    }
}

/// Indicates the protocol in use. Only one protocol is supported.
#[derive(Debug, Clone, Default)]
pub enum Protocol {
    /// Serializes to the value `"nu-plugin"`
    #[default]
    NuPlugin,
}

/// Indicates optional protocol features. This can help to make non-breaking-change additions to
/// the protocol. Features are not restricted to plain strings and can contain additional
/// configuration data.
///
/// Optional features should not be used by the protocol if they are not present in the
/// [`ProtocolInfo`] sent by the other side.
#[derive(Debug, Clone)]
pub enum Feature {
    /// The plugin supports running with a local socket passed via `--local-socket` instead of
    /// stdio.
    LocalSocket,

    /// A feature that was not recognized on deserialization. Attempting to serialize this feature
    /// is an error. Matching against it may only be used if necessary to determine whether
    /// unsupported features are present.
    Unknown,
}

impl Feature {
    /// True if the feature is considered to be compatible with another feature.
    pub fn is_compatible_with(&self, other: &Feature) -> bool {
        matches!((self, other), (Feature::LocalSocket, Feature::LocalSocket))
    }
}

/// Protocol features compiled into this version of `nu-plugin`.
pub fn default_features() -> Vec<Feature> {
    vec![
        // Only available if compiled with the `local-socket` feature flag (enabled by default).
        #[cfg(feature = "local-socket")]
        Feature::LocalSocket,
    ]
}
